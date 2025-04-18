import type { MCPToolStakeLevelType } from "@app/lib/actions/constants";
import { getResourceNameAndIdFromSId } from "@app/lib/resources/string_ids";
import type { ModelId, Result, WhitelistableFeature } from "@app/types";
import { Err, Ok } from "@app/types";

export const AVAILABLE_INTERNAL_MCP_SERVER_NAMES = [
  // Note:
  // Names should reflect the purpose of the server, but not directly the tools it contains.
  // We'll prefix all tools with the server name to avoid conflicts.
  // It's okay to change the name of the server as we don't refer to it directly.
  "image_generator",
  "file_generator",
  "github",
  "data_sources_debugger",
  "authentication_debugger",
  "tables_debugger",
  "child_agent_debugger",
] as const;

export const INTERNAL_MCP_SERVERS: Record<
  InternalMCPServerNameType,
  {
    id: number;
    isDefault: boolean;
    flag: WhitelistableFeature | null;
  }
> = {
  // Notes:
  // ids should be stable, do not change them for production internal servers as it would break existing agents.
  // Let's start dev actions at 1000 to avoid conflicts with production actions.
  // flag "mcp_actions" for actions that are part of the MCP actions feature.
  // flag "dev_mcp_actions" for actions that are only used internally for dev and testing.

  // Production
  github: {
    id: 1,
    isDefault: false,
    flag: "mcp_actions",
  },
  image_generator: {
    id: 2,
    isDefault: true,
    flag: "mcp_actions",
  },
  file_generator: {
    id: 3,
    isDefault: true,
    flag: "mcp_actions",
  },

  // Dev
  data_sources_debugger: {
    id: 1000,
    isDefault: true,
    flag: "dev_mcp_actions",
  },
  child_agent_debugger: {
    id: 1001,
    isDefault: false,
    flag: "dev_mcp_actions",
  },
  authentication_debugger: {
    id: 1002,
    isDefault: false,
    flag: "dev_mcp_actions",
  },
  tables_debugger: {
    id: 1003,
    isDefault: false,
    flag: "dev_mcp_actions",
  },
};

export const INTERNAL_TOOLS_STAKE_LEVEL: Partial<
  Record<InternalMCPServerNameType, Record<string, MCPToolStakeLevelType>>
> = {
  authentication_debugger: {
    hello_world: "low",
  },
};

export type InternalMCPServerNameType =
  (typeof AVAILABLE_INTERNAL_MCP_SERVER_NAMES)[number];

export const isDefaultInternalMCPServerByName = (
  name: InternalMCPServerNameType
): boolean => {
  return INTERNAL_MCP_SERVERS[name].isDefault;
};

export const isDefaultInternalMCPServer = (sId: string): boolean => {
  const r = getInternalMCPServerNameAndWorkspaceId(sId);
  if (r.isErr()) {
    return false;
  }
  return isDefaultInternalMCPServerByName(r.value.name);
};

export const getInternalMCPServerNameAndWorkspaceId = (
  sId: string
): Result<
  {
    name: InternalMCPServerNameType;
    workspaceId: ModelId;
  },
  Error
> => {
  const sIdParts = getResourceNameAndIdFromSId(sId);

  if (!sIdParts) {
    return new Err(new Error(`Invalid internal MCPServer sId: ${sId}`));
  }

  if (sIdParts.resourceName !== "internal_mcp_server") {
    return new Err(
      new Error(
        `Invalid internal MCPServer sId: ${sId}, does not refer to an internal MCP server.`
      )
    );
  }

  // Swap keys and values.
  const details = Object.entries(INTERNAL_MCP_SERVERS).find(
    ([, internalMCPServer]) => internalMCPServer.id === sIdParts.resourceId
  );

  if (!details) {
    return new Err(
      new Error(
        `Invalid internal MCPServer sId: ${sId}, ID does not match any known internal MCPServer.`
      )
    );
  }

  if (!isInternalMCPServerName(details[0])) {
    return new Err(
      new Error(`Invalid internal MCPServer name: ${details[0]}, sId: ${sId}`)
    );
  }

  const name = details[0];

  return new Ok({
    name,
    workspaceId: sIdParts.workspaceId,
  });
};

export const isInternalMCPServerName = (
  name: string
): name is InternalMCPServerNameType =>
  AVAILABLE_INTERNAL_MCP_SERVER_NAMES.includes(
    name as InternalMCPServerNameType
  );

export const isValidInternalMCPServerId = (
  workspaceId: ModelId,
  sId: string
): boolean => {
  const r = getInternalMCPServerNameAndWorkspaceId(sId);
  if (r.isOk()) {
    return r.value.workspaceId === workspaceId;
  }

  return false;
};
