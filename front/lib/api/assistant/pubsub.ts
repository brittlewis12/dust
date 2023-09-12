import {
  AgentActionEvent,
  AgentActionSuccessEvent,
  AgentErrorEvent,
  AgentGenerationSuccessEvent,
} from "@app/lib/api/assistant/agent";
import {
  AgentMessageNewEvent,
  postUserMessage,
  UserMessageNewEvent,
} from "@app/lib/api/assistant/conversation";
import { GenerationTokensEvent } from "@app/lib/api/assistant/generation";
import { Authenticator } from "@app/lib/auth";
import { redisClient } from "@app/lib/redis";
import logger from "@app/logger/logger";
import {
  ConversationType,
  MentionType,
  UserMessageContext,
  UserMessageType,
} from "@app/types/assistant/conversation";

export async function postUserMessageWithPubSub(
  auth: Authenticator,
  {
    conversation,
    content,
    mentions,
    context,
  }: {
    conversation: ConversationType;
    content: string;
    mentions: MentionType[];
    context: UserMessageContext;
  }
): Promise<UserMessageType> {
  const promise: Promise<UserMessageType> = new Promise((resolve, reject) => {
    void (async () => {
      const redis = await redisClient();
      let didResolve = false;
      try {
        for await (const event of postUserMessage(auth, {
          conversation,
          content,
          mentions,
          context,
        })) {
          switch (event.type) {
            case "user_message_new":
            case "agent_message_new": {
              const pubsubChannel = getConversationChannelId(conversation.sId);
              await redis.xAdd(pubsubChannel, "*", {
                payload: JSON.stringify(event),
              });
              if (event.type === "user_message_new") {
                didResolve = true;
                resolve(event.message);
              }
              break;
            }
            case "retrieval_params":
            case "agent_error":
            case "agent_action_success":
            case "retrieval_documents":
            case "generation_tokens":
            case "agent_generation_success":
            case "agent_message_success": {
              const pubsubChannel = getMessageChannelId(event.messageId);
              await redis.xAdd(pubsubChannel, "*", {
                payload: JSON.stringify(event),
              });
              break;
            }
            case "user_message_error": {
              // We reject the promise here which means we'll get a 500 in the route calling
              // postUserMessageWithPubSub. This is fine since `user_message_error` can only happen
              // if we're trying to send a message to a conversation that we don't have access to,
              // or this has already been checked if getConversation has been called.
              reject(new Error(event.error.message));
              break;
            }

            default:
              ((blockParent: never) => {
                logger.error("Unknown event type", blockParent);
              })(event);
              return null;
          }
        }
      } catch (e) {
        logger.error({ error: e }, "Error Posting message");
      } finally {
        await redis.quit();
        if (!didResolve) {
          reject(
            new Error(
              `Never got the user_message_new event for ${conversation.sId}`
            )
          );
        }
      }
    })();
  });

  return promise;
}

export async function* getConversationEvents(
  conversationId: string,
  lastEventId: string | null
): AsyncGenerator<{
  eventId: string;
  data: UserMessageNewEvent | AgentMessageNewEvent;
}> {
  const redis = await redisClient();
  const pubsubChannel = getConversationChannelId(conversationId);

  try {
    while (true) {
      const events = await redis.xRead(
        { key: pubsubChannel, id: lastEventId ? lastEventId : "0-0" },
        // weird, xread does not return on new message when count is = 1. Anything over 1 works.
        { COUNT: 1, BLOCK: 60 * 1000 }
      );
      if (!events) {
        return;
      }
      for (const event of events) {
        for (const message of event.messages) {
          const payloadStr = message.message["payload"];
          const messageId = message.id;
          const payload = JSON.parse(payloadStr);
          lastEventId = messageId;
          yield {
            eventId: messageId,
            data: payload,
          };
        }
      }
    }
  } finally {
    await redis.quit();
  }
}

export async function* getMessagesEvents(
  messageId: string,
  lastEventId: string | null
): AsyncGenerator<{
  eventId: string;
  data:
    | AgentErrorEvent
    | AgentActionEvent
    | AgentActionSuccessEvent
    | GenerationTokensEvent
    | AgentGenerationSuccessEvent;
}> {
  const pubsubChannel = getMessageChannelId(messageId);
  const client = await redisClient();
  const events = await client.xRead(
    { key: pubsubChannel, id: lastEventId ? lastEventId : "0-0" },
    { COUNT: 1, BLOCK: 60 * 1000 }
  );
  if (!events) {
    return;
  }
  for (const event of events) {
    for (const message of event.messages) {
      const payloadStr = message.message["payload"];
      const messageId = message.id;
      const payload = JSON.parse(payloadStr);
      yield {
        eventId: messageId,
        data: payload,
      };
    }
  }
}

function getConversationChannelId(channelId: string) {
  return `conversation-${channelId}`;
}

function getMessageChannelId(messageId: string) {
  return `message-${messageId}`;
}
