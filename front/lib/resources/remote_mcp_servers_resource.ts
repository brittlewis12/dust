import assert from "assert";
import type {
  Attributes,
  CreationAttributes,
  ModelStatic,
  Transaction,
} from "sequelize";

import type { MCPServerType, MCPToolType } from "@app/lib/actions/mcp_metadata";
import type { Authenticator } from "@app/lib/auth";
import { MCPServerView } from "@app/lib/models/assistant/actions/mcp_server_view";
import { RemoteMCPServer } from "@app/lib/models/assistant/actions/remote_mcp_server";
import { ResourceWithSpace } from "@app/lib/resources/resource_with_space";
import { SpaceResource } from "@app/lib/resources/space_resource";
import type { ReadonlyAttributesType } from "@app/lib/resources/storage/types";
import { getResourceIdFromSId, makeSId } from "@app/lib/resources/string_ids";
import type { ResourceFindOptions } from "@app/lib/resources/types";
import type { ModelId, Result } from "@app/types";
import { Ok, removeNulls } from "@app/types";

// Attributes are marked as read-only to reflect the stateless nature of our Resource.
// eslint-disable-next-line @typescript-eslint/no-empty-interface, @typescript-eslint/no-unsafe-declaration-merging
export interface RemoteMCPServerResource
  extends ReadonlyAttributesType<RemoteMCPServer> {}
// eslint-disable-next-line @typescript-eslint/no-unsafe-declaration-merging
export class RemoteMCPServerResource extends ResourceWithSpace<RemoteMCPServer> {
  static model: ModelStatic<RemoteMCPServer> = RemoteMCPServer;

  constructor(
    model: ModelStatic<RemoteMCPServer>,
    blob: Attributes<RemoteMCPServer>,
    public readonly space: SpaceResource
  ) {
    super(RemoteMCPServer, blob, space);
  }

  static async makeNew(
    auth: Authenticator,
    blob: Omit<CreationAttributes<RemoteMCPServer>, "spaceId" | "sId">,
    transaction?: Transaction
  ) {
    const systemSpace = await SpaceResource.fetchWorkspaceSystemSpace(auth);

    assert(
      systemSpace.canWrite(auth),
      "The user is not authorized to create an MCP server"
    );

    const server = await RemoteMCPServer.create(
      {
        ...blob,
        vaultId: systemSpace.id,
      },
      { transaction }
    );

    // Immediately create a view for the server in the system space.
    await MCPServerView.create(
      {
        workspaceId: auth.getNonNullableWorkspace().id,
        serverType: "remote",
        remoteMCPServerId: server.id,
        vaultId: systemSpace.id,
        editedAt: new Date(),
        editedByUserId: auth.user()?.id,
      },
      {
        transaction,
      }
    );

    return new this(RemoteMCPServer, server.get(), systemSpace);
  }

  // Fetching.

  private static async baseFetch(
    auth: Authenticator,
    options?: ResourceFindOptions<RemoteMCPServer>
  ) {
    const servers = await this.baseFetchWithAuthorization(auth, {
      ...options,
    });
    return servers.filter((server) => auth.isAdmin() || server.canRead(auth));
  }

  static async fetchByIds(
    auth: Authenticator,
    ids: string[]
  ): Promise<RemoteMCPServerResource[]> {
    return this.baseFetch(auth, {
      where: {
        id: removeNulls(ids.map(getResourceIdFromSId)),
      },
    });
  }

  static async fetchById(
    auth: Authenticator,
    id: string
  ): Promise<RemoteMCPServerResource | null> {
    const [server] = await this.fetchByIds(auth, [id]);
    return server ?? null;
  }

  static async findByPk(
    auth: Authenticator,
    id: number,
    options?: ResourceFindOptions<RemoteMCPServer>
  ): Promise<RemoteMCPServerResource | null> {
    const servers = await this.baseFetch(auth, {
      where: {
        id,
      },
      ...options,
    });
    return servers.length > 0 ? servers[0] : null;
  }

  static async listByWorkspace(
    auth: Authenticator,
    options?: { includeDeleted?: boolean }
  ) {
    return this.baseFetch(auth, {
      where: {
        workspaceId: auth.getNonNullableWorkspace().id,
      },
      ...options,
    });
  }

  static async findByUrl(
    auth: Authenticator,
    url: string,
    { includeDeleted }: { includeDeleted?: boolean } = {}
  ) {
    const servers = await this.baseFetch(auth, {
      where: {
        url,
      },
      includeDeleted,
    });

    return servers.length > 0 ? servers[0] : null;
  }

  // sId
  get sId(): string {
    return RemoteMCPServerResource.modelIdToSId({
      id: this.id,
      workspaceId: this.workspaceId,
    });
  }

  static modelIdToSId({
    id,
    workspaceId,
  }: {
    id: ModelId;
    workspaceId: ModelId;
  }): string {
    return makeSId("remote_mcp_server", {
      id,
      workspaceId,
    });
  }

  // Deletion.

  async hardDelete(
    auth: Authenticator,
    transaction?: Transaction
  ): Promise<Result<number, Error>> {
    assert(
      this.canWrite(auth),
      "The user is not authorized to delete this MCP server"
    );

    // Directly delete the DataSourceViewModel here to avoid a circular dependency.
    await MCPServerView.destroy({
      where: {
        workspaceId: auth.getNonNullableWorkspace().id,
        remoteMCPServerId: this.id,
      },
      transaction,
      // Use 'hardDelete: true' to ensure the record is permanently deleted from the database,
      // bypassing the soft deletion in place.
      hardDelete: true,
    });

    const deletedCount = await RemoteMCPServer.destroy({
      where: {
        workspaceId: auth.getNonNullableWorkspace().id,
        id: this.id,
      },
      transaction,
      hardDelete: true,
    });

    return new Ok(deletedCount);
  }

  async softDelete(
    auth: Authenticator,
    transaction?: Transaction
  ): Promise<Result<number, Error>> {
    assert(
      this.canWrite(auth),
      "The user is not authorized to delete this MCP server"
    );
    const deletedCount = await RemoteMCPServer.destroy({
      where: {
        workspaceId: auth.getNonNullableWorkspace().id,
        id: this.id,
      },
      transaction,
      hardDelete: false,
    });

    return new Ok(deletedCount);
  }

  // Mutation.

  async updateMetadata(
    auth: Authenticator,
    {
      name,
      description,
      url,
      sharedSecret,
      cachedTools,
      lastSyncAt,
    }: {
      name?: string;
      description?: string | null;
      url?: string;
      sharedSecret?: string;
      cachedTools: MCPToolType[];
      lastSyncAt: Date;
    }
  ) {
    assert(
      this.canWrite(auth),
      "The user is not authorized to update this MCP server"
    );
    await this.update({
      name,
      description,
      url,
      sharedSecret,
      cachedTools,
      lastSyncAt,
    });
  }

  // Serialization.
  toJSON(): MCPServerType & {
    // Remote MCP Server specifics
    url: string;
    lastSyncAt: Date | null;
    sharedSecret: string;
  } {
    return {
      id: this.sId,
      name: this.name,
      description: this.description || "",
      tools: this.cachedTools,
      // TODO(mcp) remove this once we have a real version & icon
      version: "0.0.1",
      icon: "rocket",

      // Remote MCP Server specifics
      url: this.url,
      lastSyncAt: this.lastSyncAt,
      sharedSecret: this.sharedSecret,
    };
  }
}
