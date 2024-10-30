import { makeDocumentCitation } from "@app/components/actions/retrieval/utils";
import { makeWebsearchResultsCitation } from "@app/components/actions/websearch/utils";
import type { MarkdownCitation } from "@app/components/assistant/markdown/MarkdownCitation";
import { RenderMessageMarkdown } from "@app/components/assistant/markdown/RenderMessageMarkdown";
import { useSubmitFunction } from "@app/lib/client/utils";
import type {
  ConversationMessageEmojiSelectorProps,
  ConversationMessageSizeType,
} from "@dust-tt/sparkle";
import {
  ArrowPathIcon,
  Button,
  ChatBubbleThoughtIcon,
  Chip,
  Citation,
  ContentMessage,
  ConversationMessage,
  DocumentDuplicateIcon,
  EyeIcon,
  Popover,
} from "@dust-tt/sparkle";
import type {
  AgentActionSpecificEvent,
  AgentActionSuccessEvent,
  AgentActionType,
  AgentErrorEvent,
  AgentGenerationCancelledEvent,
  AgentMessageSuccessEvent,
  GenerationTokensEvent,
  LightWorkspaceType,
  RetrievalActionType,
  WebsearchActionType,
} from "@dust-tt/types";
import type { AgentMessageType } from "@dust-tt/types";
import {
  assertNever,
  isRetrievalActionType,
  isWebsearchActionType,
  removeNulls,
} from "@dust-tt/types";
import { useEventSource } from "@extension/hooks/useEventSource";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

interface AgentMessageProps {
  conversationId: string;
  isLastMessage: boolean;
  message: AgentMessageType;
  messageEmoji?: ConversationMessageEmojiSelectorProps;
  owner: LightWorkspaceType;
  size: ConversationMessageSizeType;
}

/**
 *
 * @param isInModal is the conversation happening in a side modal, i.e. when
 * testing an assistant? see conversation/Conversation.tsx
 * @returns
 */
export function AgentMessage({
  conversationId,
  isLastMessage,
  message,
  messageEmoji,
  owner,
  size,
}: AgentMessageProps) {
  const [streamedAgentMessage, setStreamedAgentMessage] =
    useState<AgentMessageType>(message);

  const [references, setReferences] = useState<{
    [key: string]: MarkdownCitation;
  }>({});

  const [activeReferences, setActiveReferences] = useState<
    { index: number; document: MarkdownCitation }[]
  >([]);

  const shouldStream = (() => {
    if (message.status !== "created") {
      return false;
    }

    switch (streamedAgentMessage.status) {
      case "succeeded":
      case "failed":
      case "cancelled":
        return false;
      case "created":
        return true;
      default:
        assertNever(streamedAgentMessage.status);
    }
  })();

  const [lastTokenClassification, setLastTokenClassification] = useState<
    null | "tokens" | "chain_of_thought"
  >(null);

  const buildEventSourceURL = useCallback(
    (lastEvent: string | null) => {
      const esURL = `${process.env.DUST_DOMAIN}/api/v1/w/${owner.sId}/assistant/conversations/${conversationId}/messages/${message.sId}/events`;
      let lastEventId = "";
      if (lastEvent) {
        const eventPayload: {
          eventId: string;
        } = JSON.parse(lastEvent);
        lastEventId = eventPayload.eventId;
      }
      const url = esURL + "?lastEventId=" + lastEventId;

      return url;
    },
    [conversationId, message.sId, owner.sId]
  );

  const onEventCallback = useCallback((eventStr: string) => {
    const eventPayload: {
      eventId: string;
      data:
        | AgentErrorEvent
        | AgentActionSpecificEvent
        | AgentActionSuccessEvent
        | GenerationTokensEvent
        | AgentGenerationCancelledEvent
        | AgentMessageSuccessEvent;
    } = JSON.parse(eventStr);

    const updateMessageWithAction = (
      m: AgentMessageType,
      action: AgentActionType
    ): AgentMessageType => {
      return {
        ...m,
        actions: m.actions
          ? [...m.actions.filter((a) => a.id !== action.id), action]
          : [action],
      };
    };

    const event = eventPayload.data;
    switch (event.type) {
      case "agent_action_success":
        setStreamedAgentMessage((m) => {
          return { ...updateMessageWithAction(m, event.action) };
        });
        break;
      case "retrieval_params":
      case "dust_app_run_params":
      case "dust_app_run_block":
      case "tables_query_started":
      case "tables_query_model_output":
      case "tables_query_output":
      case "process_params":
      case "websearch_params":
      case "browse_params":
        setStreamedAgentMessage((m) => {
          return updateMessageWithAction(m, event.action);
        });
        break;
      case "agent_error":
        setStreamedAgentMessage((m) => {
          return { ...m, status: "failed", error: event.error };
        });
        break;

      case "agent_generation_cancelled":
        setStreamedAgentMessage((m) => {
          return { ...m, status: "cancelled" };
        });
        break;
      case "agent_message_success": {
        setStreamedAgentMessage((m) => {
          return {
            ...m,
            ...event.message,
          };
        });
        break;
      }

      case "generation_tokens": {
        switch (event.classification) {
          case "closing_delimiter":
            break;
          case "opening_delimiter":
            break;
          case "tokens":
            setLastTokenClassification("tokens");
            setStreamedAgentMessage((m) => {
              const previousContent = m.content || "";
              return { ...m, content: previousContent + event.text };
            });
            break;
          case "chain_of_thought":
            setLastTokenClassification("chain_of_thought");
            setStreamedAgentMessage((m) => {
              const currentChainOfThought = m.chainOfThought ?? "";
              return {
                ...m,
                chainOfThought: currentChainOfThought + event.text,
              };
            });
            break;
          default:
            assertNever(event);
        }
        break;
      }

      default:
        assertNever(event);
    }
  }, []);

  useEventSource(
    buildEventSourceURL,
    onEventCallback,
    `message-${message.sId}`,
    { isReadyToConsumeStream: shouldStream }
  );

  const agentMessageToRender = ((): AgentMessageType => {
    switch (message.status) {
      case "succeeded":
      case "failed":
        return message;
      case "cancelled":
        if (streamedAgentMessage.status === "created") {
          return { ...streamedAgentMessage, status: "cancelled" };
        }
        return message;
      case "created":
        return streamedAgentMessage;
      default:
        assertNever(message.status);
    }
  })();

  // Autoscroll is performed when a message is generating and the page is
  // already scrolled down; but if the user has scrolled the page up after the
  // start of the message, we do not want to scroll it back down.
  //
  // Checking the conversation is already at the bottom of the screen is done
  // modulo a small margin (50px). This value is small because if large, it
  // prevents user from scrolling up when the message continues generating
  // (forces it back down), but it cannot be zero otherwise the scroll does not
  // happen.
  const isAtBottom = useRef(true);
  const bottomRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        isAtBottom.current = entry.isIntersecting;
      },
      { threshold: 1 }
    );

    const currentBottomRef = bottomRef.current;

    if (currentBottomRef) {
      observer.observe(currentBottomRef);
    }

    return () => {
      if (currentBottomRef) {
        observer.unobserve(currentBottomRef);
      }
    };
  }, []);

  // References logic.
  function updateActiveReferences(document: MarkdownCitation, index: number) {
    const existingIndex = activeReferences.find((r) => r.index === index);
    if (!existingIndex) {
      setActiveReferences([...activeReferences, { index, document }]);
    }
  }

  const [lastHoveredReference, setLastHoveredReference] = useState<
    number | null
  >(null);

  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (lastHoveredReference !== null) {
      timer = setTimeout(() => {
        setLastHoveredReference(null);
      }, 1000); // Reset after 1 second.
    }
    return () => clearTimeout(timer);
  }, [lastHoveredReference]);

  useEffect(() => {
    // Retrieval actions
    const retrievalActionsWithDocs = agentMessageToRender.actions
      .filter((a) => isRetrievalActionType(a) && a.documents)
      .sort((a, b) => a.id - b.id) as RetrievalActionType[];
    const allDocs = removeNulls(
      retrievalActionsWithDocs.map((a) => a.documents).flat()
    );
    const allDocsReferences = allDocs.reduce<{
      [key: string]: MarkdownCitation;
    }>((acc, d) => {
      acc[d.reference] = makeDocumentCitation(d);
      return acc;
    }, {});

    // Websearch actions
    const websearchActionsWithResults = agentMessageToRender.actions
      .filter((a) => isWebsearchActionType(a) && a.output?.results?.length)
      .sort((a, b) => a.id - b.id) as WebsearchActionType[];
    const allWebResults = removeNulls(
      websearchActionsWithResults.map((a) => a.output?.results).flat()
    );
    const allWebReferences = allWebResults.reduce<{
      [key: string]: MarkdownCitation;
    }>((acc, l) => {
      acc[l.reference] = makeWebsearchResultsCitation(l);
      return acc;
    }, {});

    // Merge all references
    setReferences({ ...allDocsReferences, ...allWebReferences });
  }, [
    agentMessageToRender.actions,
    agentMessageToRender.status,
    agentMessageToRender.sId,
  ]);
  const { configuration: agentConfiguration } = agentMessageToRender;

  const citations = useMemo(
    () => getCitations({ activeReferences, lastHoveredReference }),
    [activeReferences, lastHoveredReference]
  );

  return (
    <ConversationMessage
      pictureUrl={agentConfiguration.pictureUrl}
      name={`@${agentConfiguration.name}`}
      buttons={[]}
      avatarBusy={agentMessageToRender.status === "created"}
      messageEmoji={messageEmoji}
      renderName={() => {
        return (
          <div className="flex flex-row items-center gap-2">
            <div className="text-base font-medium">
              {/* TODO(Ext) Any CTA here ? */}
              {agentConfiguration.name}
            </div>
          </div>
        );
      }}
      type="agent"
      size={size}
      citations={citations}
    >
      <div>
        {renderAgentMessage({
          agentMessage: agentMessageToRender,
          references: references,
          streaming: shouldStream,
          lastTokenClassification: lastTokenClassification,
        })}
      </div>
      {/* Invisible div to act as a scroll anchor for detecting when the user has scrolled to the bottom */}
      <div ref={bottomRef} className="h-1.5" />
    </ConversationMessage>
  );

  function renderAgentMessage({
    agentMessage,
    references,
    streaming,
    lastTokenClassification,
  }: {
    agentMessage: AgentMessageType;
    references: { [key: string]: MarkdownCitation };
    streaming: boolean;
    lastTokenClassification: null | "tokens" | "chain_of_thought";
  }) {
    if (agentMessage.status === "failed") {
      return (
        <ErrorMessage
          error={
            agentMessage.error || {
              message: "Unexpected Error",
              code: "unexpected_error",
            }
          }
          retryHandler={() => {}}
        />
      );
    }

    return (
      <div className="flex flex-col gap-y-4">
        {/* TODO(Ext): Tools inspection */}
        {/* <AgentMessageActions
          agentMessage={agentMessage}
          size={size}
          owner={owner}
        /> */}

        {agentMessage.chainOfThought?.length ? (
          <ContentMessage
            title="Assistant thoughts"
            variant="purple"
            icon={ChatBubbleThoughtIcon}
          >
            <RenderMessageMarkdown
              content={agentMessage.chainOfThought}
              isStreaming={false}
              textSize="sm"
              textColor="purple-800"
              isLastMessage={isLastMessage}
            />
          </ContentMessage>
        ) : null}

        {agentMessage.content !== null && (
          <div>
            {lastTokenClassification !== "chain_of_thought" &&
            agentMessage.content === "" ? (
              <div className="blinking-cursor">
                <span></span>
              </div>
            ) : (
              <>
                <RenderMessageMarkdown
                  content={agentMessage.content}
                  isStreaming={
                    streaming && lastTokenClassification === "tokens"
                  }
                  citationsContext={{
                    references,
                    updateActiveReferences,
                    setHoveredReference: setLastHoveredReference,
                  }}
                  isLastMessage={isLastMessage}
                />
              </>
            )}
          </div>
        )}
        {agentMessage.status === "cancelled" && (
          <Chip
            label="Message generation was interrupted"
            size="xs"
            className="mt-4"
          />
        )}
      </div>
    );
  }

  // TODO(Ext): Handle retry API
  // async function retryHandler(agentMessage: AgentMessageType) {
  //   setIsRetryHandlerProcessing(true);
  //   await fetch(
  //     `/api/w/${owner.sId}/assistant/conversations/${conversationId}/messages/${agentMessage.sId}/retry`,
  //     {
  //       method: "POST",
  //       headers: {
  //         "Content-Type": "application/json",
  //       },
  //     }
  //   );
  //   setIsRetryHandlerProcessing(false);
  // }
}

function getCitations({
  activeReferences,
  lastHoveredReference,
}: {
  activeReferences: {
    index: number;
    document: MarkdownCitation;
  }[];
  lastHoveredReference: number | null;
}) {
  activeReferences.sort((a, b) => a.index - b.index);
  return activeReferences.map(({ document, index }) => {
    return (
      <Citation
        key={index}
        size="xs"
        sizing="fluid"
        isBlinking={lastHoveredReference === index}
        type={document.type}
        title={document.title}
        href={document.href}
        index={index}
      />
    );
  });
}

function ErrorMessage({
  error,
  retryHandler,
}: {
  error: { code: string; message: string };
  retryHandler: () => void;
}) {
  const fullMessage =
    "ERROR: " + error.message + (error.code ? ` (code: ${error.code})` : "");

  const { submit: retry, isSubmitting: isRetrying } = useSubmitFunction(
    async () => retryHandler()
  );

  return (
    <div className="flex flex-col gap-9">
      <div className="flex flex-col gap-1 sm:flex-row">
        <Chip
          color="warning"
          label={"ERROR: " + shortText(error.message)}
          size="xs"
        />
        <Popover
          trigger={
            <Button
              variant="ghost"
              size="xs"
              icon={EyeIcon}
              label="See the error"
            />
          }
          content={
            <div className="flex flex-col gap-3">
              <div className="text-sm font-normal text-warning-800">
                {fullMessage}
              </div>
              <div className="self-end">
                <Button
                  variant="ghost"
                  size="xs"
                  icon={DocumentDuplicateIcon}
                  label={"Copy"}
                  onClick={() =>
                    void navigator.clipboard.writeText(fullMessage)
                  }
                />
              </div>
            </div>
          }
        />
      </div>
      <div>
        <Button
          variant="ghost"
          size="sm"
          icon={ArrowPathIcon}
          label="Retry"
          onClick={retry}
          disabled={isRetrying}
        />
      </div>
    </div>
  );
}

function shortText(text: string, maxLength = 30) {
  return text.length > maxLength ? text.substring(0, maxLength) + "..." : text;
}
