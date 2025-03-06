import type { ContentNode, Result } from "@dust-tt/types";
import { Err, MIME_TYPES, Ok } from "@dust-tt/types";

import { makeGongTranscriptFolderInternalId } from "@connectors/connectors/gong/lib/internal_ids";
import { baseUrlFromConnectionId } from "@connectors/connectors/gong/lib/oauth";
import {
  fetchGongConfiguration,
  fetchGongConnector,
} from "@connectors/connectors/gong/lib/utils";
import {
  launchGongSyncWorkflow,
  stopGongSyncWorkflow,
} from "@connectors/connectors/gong/temporal/client";
import type {
  CreateConnectorErrorCode,
  RetrievePermissionsErrorCode,
  UpdateConnectorErrorCode,
} from "@connectors/connectors/interface";
import { ConnectorManagerError } from "@connectors/connectors/interface";
import { BaseConnectorManager } from "@connectors/connectors/interface";
import { dataSourceConfigFromConnector } from "@connectors/lib/api/data_source_config";
import { upsertDataSourceFolder } from "@connectors/lib/data_sources";
import logger from "@connectors/logger/logger";
import { ConnectorResource } from "@connectors/resources/connector_resource";
import { GongConfigurationResource } from "@connectors/resources/gong_resources";
import type { DataSourceConfig } from "@connectors/types/data_source_config";

const TRANSCRIPTS_FOLDER_TITLE = "Transcripts";

export class GongConnectorManager extends BaseConnectorManager<null> {
  static async create({
    dataSourceConfig,
    connectionId,
  }: {
    dataSourceConfig: DataSourceConfig;
    connectionId: string;
  }): Promise<Result<string, ConnectorManagerError<CreateConnectorErrorCode>>> {
    const baseUrlRes = await baseUrlFromConnectionId(connectionId);
    if (baseUrlRes.isErr()) {
      throw new Error("Invalid Gong Access Token");
    }

    const connector = await ConnectorResource.makeNew(
      "gong",
      {
        connectionId,
        workspaceAPIKey: dataSourceConfig.workspaceAPIKey,
        workspaceId: dataSourceConfig.workspaceId,
        dataSourceId: dataSourceConfig.dataSourceId,
      },
      {
        baseUrl: baseUrlRes.value,
      }
    );

    // Upsert a top-level folder that will contain all the transcripts (non selectable).
    await upsertDataSourceFolder({
      dataSourceConfig: dataSourceConfigFromConnector(connector),
      folderId: makeGongTranscriptFolderInternalId(connector),
      parents: [makeGongTranscriptFolderInternalId(connector)],
      parentId: null,
      title: TRANSCRIPTS_FOLDER_TITLE,
      mimeType: MIME_TYPES.GONG.TRANSCRIPT_FOLDER,
    });

    const result = await launchGongSyncWorkflow(connector);
    if (result.isErr()) {
      logger.error(
        { connectorId: connector.id, error: result.error },
        "[Gong] Error launching Gong sync workflow"
      );
      throw result.error;
    }

    return new Ok(connector.id.toString());
  }

  async update({
    connectionId,
  }: {
    connectionId?: string | null;
  }): Promise<Result<string, ConnectorManagerError<UpdateConnectorErrorCode>>> {
    const connector = await fetchGongConnector({
      connectorId: this.connectorId,
    });

    if (connectionId) {
      const config = await fetchGongConfiguration(connector);

      const { baseUrl } = config;
      const newBaseUrlRes = await baseUrlFromConnectionId(connectionId);

      if (newBaseUrlRes.isErr()) {
        throw new Error("Invalid Gong Access Token");
      }

      if (newBaseUrlRes.value !== baseUrl) {
        return new Err(
          new ConnectorManagerError(
            "CONNECTOR_OAUTH_TARGET_MISMATCH",
            "Cannot change workspace of a Gong connector"
          )
        );
      }

      await connector.update({
        connectionId,
      });

      // If connector was previously paused, unpause it.
      if (connector.isPaused()) {
        await this.unpause();

        await launchGongSyncWorkflow(connector);
      }
    }

    return new Ok(connector.id.toString());
  }

  async clean(): Promise<Result<undefined, Error>> {
    const { connectorId } = this;
    const connector = await ConnectorResource.fetchById(connectorId);
    if (!connector) {
      logger.error({ connectorId }, "[Gong] Connector not found.");
      return new Err(new Error("[Gong] Connector not found"));
    }

    const res = await connector.delete();
    if (res.isErr()) {
      logger.error(
        { connectorId, error: res.error },
        "Error cleaning up Gong connector."
      );
      return res;
    }

    return new Ok(undefined);
  }

  async stop(): Promise<Result<undefined, Error>> {
    const { connectorId } = this;
    const connector = await ConnectorResource.fetchById(connectorId);
    if (!connector) {
      logger.error({ connectorId }, "[Gong] Connector not found.");
      throw new Error("[Gong] Connector not found.");
    }
    const result = await stopGongSyncWorkflow(connector);
    if (result.isErr()) {
      return result;
    }
    return new Ok(undefined);
  }

  async resume(): Promise<Result<undefined, Error>> {
    const connector = await ConnectorResource.fetchById(this.connectorId);
    if (!connector) {
      throw new Error(
        `[Gong] Connector not found. ConnectorId: ${this.connectorId}`
      );
    }

    const result = await launchGongSyncWorkflow(connector);
    if (result.isErr()) {
      logger.error(
        { connectorId: this.connectorId, error: result.error },
        "[Gong] Error launching Gong sync workflow"
      );
      throw result.error;
    }

    return new Ok(undefined);
  }

  async sync({
    fromTs,
  }: {
    fromTs: number | null;
  }): Promise<Result<string, Error>> {
    const connector = await ConnectorResource.fetchById(this.connectorId);
    if (!connector) {
      throw new Error("[Gong] Connector not found.");
    }
    const configuration =
      await GongConfigurationResource.fetchByConnector(connector);
    if (!configuration) {
      throw new Error("[Gong] Configuration not found.");
    }
    if (!fromTs) {
      // Resetting the last sync timestamp to run a full sync.
      await configuration.resetLastSyncTimestamp();
    } else {
      // If fromTs is set, we ignore it and sync from the last cursor; we cannot miss transcripts if we assume that
      // transcripts cannot be created in the past.
      logger.warn(
        `[Gong] Ignoring the fromTs, syncing from ${configuration.lastSyncTimestamp}`
      );
    }

    const result = await launchGongSyncWorkflow(connector);
    if (result.isErr()) {
      logger.error(
        { connectorId: this.connectorId, error: result.error },
        "[Gong] Error launching Gong sync workflow"
      );
      throw result.error;
    }
    return new Ok(this.connectorId.toString());
  }

  async retrievePermissions(): Promise<
    Result<ContentNode[], ConnectorManagerError<RetrievePermissionsErrorCode>>
  > {
    return new Ok([]);
  }

  async setPermissions(): Promise<Result<void, Error>> {
    throw new Error("Method not supported.");
  }

  async pause(): Promise<Result<undefined, Error>> {
    const { connectorId } = this;
    const connector = await ConnectorResource.fetchById(connectorId);
    if (!connector) {
      logger.error({ connectorId }, "[Gong] Connector not found.");
      throw new Error("[Gong] Connector not found.");
    }
    await connector.markAsPaused();
    return this.stop();
  }

  async unpause(): Promise<Result<undefined, Error>> {
    const { connectorId } = this;
    const connector = await ConnectorResource.fetchById(connectorId);
    if (!connector) {
      logger.error({ connectorId }, "[Gong] Connector not found.");
      throw new Error("[Gong] Connector not found.");
    }
    await connector.markAsUnpaused();
    return this.resume();
  }

  async garbageCollect(): Promise<Result<string, Error>> {
    throw new Error("Method not supported.");
  }

  async configure(): Promise<Result<void, Error>> {
    throw new Error("Method not supported.");
  }

  async setConfigurationKey(): Promise<Result<void, Error>> {
    throw new Error("Method not supported.");
  }

  async getConfigurationKey(): Promise<Result<string | null, Error>> {
    throw new Error("Method not supported.");
  }
}
