import type { JSONSchema7 } from "json-schema";
import { z } from "zod";

import type {
  LocalMCPServerConfigurationType,
  LocalMCPToolConfigurationType,
  MCPServerConfigurationType,
  MCPToolConfigurationType,
  PlatformMCPServerConfigurationType,
  PlatformMCPToolConfigurationType,
} from "@app/lib/actions/mcp";
import type {
  MCPConnectionParams,
  MCPToolType,
} from "@app/lib/actions/mcp_metadata";
import {
  connectToMCPServer,
  extractMetadataFromTools,
} from "@app/lib/actions/mcp_metadata";
import type { AgentActionConfigurationType } from "@app/lib/actions/types/agent";
import {
  isMCPServerConfiguration,
  isPlatformMCPServerConfiguration,
  isPlatformMCPToolConfiguration,
} from "@app/lib/actions/types/guards";
import type { Authenticator } from "@app/lib/auth";
import { getFeatureFlags } from "@app/lib/auth";
import { MCPServerViewResource } from "@app/lib/resources/mcp_server_view_resource";
import { generateRandomModelSId } from "@app/lib/resources/string_ids";
import logger from "@app/logger/logger";
import type { Result } from "@app/types";
import { Err, normalizeError, Ok } from "@app/types";

const DEFAULT_MCP_REQUEST_TIMEOUT_MS = 60 * 1000; // 1 minute.

const EMPTY_INPUT_SCHEMA: JSONSchema7 = { type: "object", properties: {} };

// Redeclared here to avoid an issue with the zod types in the @modelcontextprotocol/sdk
// See https://github.com/colinhacks/zod/issues/2938
const ResourceContentsSchema = z.object({
  uri: z.string(),
  mimeType: z.optional(z.string()),
});

const TextResourceContentsSchema = ResourceContentsSchema.extend({
  text: z.string(),
});

const BlobResourceContentsSchema = ResourceContentsSchema.extend({
  blob: z.string().base64(),
});

const TextContentSchema = z.object({
  type: z.literal("text"),
  text: z.string(),
});

const ImageContentSchema = z.object({
  type: z.literal("image"),
  data: z.string().base64(),
  mimeType: z.string(),
});

const EmbeddedResourceSchema = z.object({
  type: z.literal("resource"),
  resource: z.union([TextResourceContentsSchema, BlobResourceContentsSchema]),
});

const Schema = z.union([
  TextContentSchema,
  ImageContentSchema,
  EmbeddedResourceSchema,
]);

export type MCPToolResultContent = z.infer<typeof Schema>;

function makePlatformMCPConfigurations(
  config: PlatformMCPServerConfigurationType,
  tools: MCPToolType[]
) {
  return tools.map((tool) => ({
    sId: generateRandomModelSId(),
    type: "mcp_configuration",
    name: tool.name,
    description: tool.description ?? null,
    inputSchema: tool.inputSchema || EMPTY_INPUT_SCHEMA,
    id: config.id,
    mcpServerViewId: config.mcpServerViewId,
    dataSources: config.dataSources || [], // Ensure dataSources is always an array
    tables: config.tables,
  }));
}

function makeLocalMCPConfigurations(
  config: LocalMCPServerConfigurationType,
  tools: MCPToolType[]
): LocalMCPToolConfigurationType[] {
  return tools.map((tool) => ({
    sId: generateRandomModelSId(),
    type: "mcp_configuration",
    name: tool.name,
    description: tool.description ?? null,
    inputSchema: tool.inputSchema || EMPTY_INPUT_SCHEMA,
    id: config.id,
    localMcpServerId: config.localMcpServerId,
  }));
}

type MCPConfigurationResult<T> = T extends PlatformMCPServerConfigurationType
  ? PlatformMCPToolConfigurationType[]
  : LocalMCPToolConfigurationType[];

function makeMCPConfigurations<T extends MCPServerConfigurationType>({
  config,
  tools,
}: {
  config: T;
  tools: MCPToolType[];
}): MCPConfigurationResult<T> {
  if (isPlatformMCPServerConfiguration(config)) {
    return makePlatformMCPConfigurations(
      config,
      tools
    ) as MCPConfigurationResult<T>;
  }

  return makeLocalMCPConfigurations(config, tools) as MCPConfigurationResult<T>;
}

/**
 * Try to call an MCP tool.
 *
 * May fail when connecting to remote/client-side servers.
 */
export async function tryCallMCPTool(
  auth: Authenticator,
  {
    conversationId,
    messageId,
    actionConfiguration,
    inputs,
  }: {
    conversationId: string;
    messageId: string;
    actionConfiguration: MCPToolConfigurationType;
    inputs: Record<string, unknown> | undefined;
  }
): Promise<Result<MCPToolResultContent[], Error>> {
  const connectionParamsRes = await getMCPClientConnectionParams(
    auth,
    actionConfiguration,
    {
      conversationId,
      messageId,
    }
  );

  if (connectionParamsRes.isErr()) {
    return connectionParamsRes;
  }

  try {
    const mcpClient = await connectToMCPServer(auth, connectionParamsRes.value);

    const toolCallResult = await mcpClient.callTool(
      {
        name: actionConfiguration.name,
        arguments: inputs,
      },
      undefined,
      { timeout: DEFAULT_MCP_REQUEST_TIMEOUT_MS }
    );

    await mcpClient.close();

    if (toolCallResult.isError) {
      return new Err(new Error(JSON.stringify(toolCallResult.content)));
    }

    // Type inference is not working here because of them using passthrough in the zod schema.
    const content: MCPToolResultContent[] = (toolCallResult.content ??
      []) as MCPToolResultContent[];

    return new Ok(content);
  } catch (error) {
    return new Err(normalizeError(error));
  }
}

async function getMCPClientConnectionParams(
  auth: Authenticator,
  config: MCPServerConfigurationType | MCPToolConfigurationType,
  {
    conversationId,
    messageId,
  }: {
    conversationId: string;
    messageId: string;
  }
): Promise<Result<MCPConnectionParams, Error>> {
  if (
    isPlatformMCPServerConfiguration(config) ||
    isPlatformMCPToolConfiguration(config)
  ) {
    const res = await MCPServerViewResource.fetchById(
      auth,
      config.mcpServerViewId
    );
    if (res.isErr()) {
      return res;
    }

    return new Ok({
      type: "mcpServerId",
      mcpServerId: res.value.mcpServerId,
    });
  }

  return new Ok({
    type: "localMCPServerId",
    mcpServerId: config.localMcpServerId,
    conversationId,
    messageId,
  });
}

/**
 * List the MCP tools for the given agent actions.
 * Returns MCP tools by connecting to the specified MCP servers.
 */
export async function tryListMCPTools(
  auth: Authenticator,
  {
    agentActions,
    conversationId,
    messageId,
  }: {
    agentActions: AgentActionConfigurationType[];
    conversationId: string;
    messageId: string;
  }
): Promise<MCPToolConfigurationType[]> {
  const owner = auth.getNonNullableWorkspace();
  const featureFlags = await getFeatureFlags(owner);
  if (!featureFlags.includes("mcp_actions")) {
    return [];
  }

  // Filter for MCP server configurations.
  const mcpServerActions = agentActions.filter(isMCPServerConfiguration);

  // Discover all the tools exposed by all the mcp servers available.
  const configurations = await Promise.all(
    mcpServerActions.map(async (action) => {
      const tools = await listMCPServerTools(auth, action, {
        conversationId,
        messageId,
      });

      return makeMCPConfigurations({
        config: action,
        tools,
      });
    })
  );

  return configurations.flat();
}

async function listMCPServerTools(
  auth: Authenticator,
  config: MCPServerConfigurationType,
  {
    conversationId,
    messageId,
  }: {
    conversationId: string;
    messageId: string;
  }
): Promise<MCPToolType[]> {
  const owner = auth.getNonNullableWorkspace();
  let mcpClient;

  const connectionParamsRes = await getMCPClientConnectionParams(auth, config, {
    conversationId,
    messageId,
  });

  if (connectionParamsRes.isErr()) {
    throw connectionParamsRes.error;
  }

  try {
    // Connect to the MCP server.
    mcpClient = await connectToMCPServer(auth, connectionParamsRes.value);

    let allTools: MCPToolType[] = [];
    let nextPageCursor;

    // Fetch all tools, handling pagination if supported by the MCP server.
    do {
      const { tools, nextCursor } = await mcpClient.listTools();
      nextPageCursor = nextCursor;
      allTools = [...allTools, ...extractMetadataFromTools(tools)];
    } while (nextPageCursor);

    logger.debug(
      {
        workspaceId: owner.id,
        conversationId,
        messageId,
        toolCount: allTools.length,
      },
      `Retrieved ${allTools.length} tools from MCP server`
    );

    return allTools;
  } catch (error) {
    logger.error(
      {
        workspaceId: owner.id,
        conversationId,
        messageId,
        error,
      },
      `Error listing tools from MCP server: ${normalizeError(error)}`
    );
    throw error;
  } finally {
    // Ensure we always close the client connection
    if (mcpClient) {
      try {
        await mcpClient.close();
      } catch (closeError) {
        logger.warn(
          {
            workspaceId: owner.id,
            conversationId,
            messageId,
            error: closeError,
          },
          "Error closing MCP client connection"
        );
      }
    }
  }
}
