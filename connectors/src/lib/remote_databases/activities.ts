import type { MIME_TYPES } from "@dust-tt/types";
import { heartbeat } from "@temporalio/activity";

import { dataSourceConfigFromConnector } from "@connectors/lib/api/data_source_config";
import {
  deleteDataSourceFolder,
  deleteDataSourceTable,
  upsertDataSourceFolder,
  upsertDataSourceRemoteTable,
} from "@connectors/lib/data_sources";
import {
  RemoteDatabaseModel,
  RemoteSchemaModel,
  RemoteTableModel,
} from "@connectors/lib/models/remote_databases";
import type { RemoteDBTree } from "@connectors/lib/remote_databases/utils";
import {
  parseSchemaInternalId,
  parseTableInternalId,
} from "@connectors/lib/remote_databases/utils";
import logger from "@connectors/logger/logger";
import type { ConnectorResource } from "@connectors/resources/connector_resource";

const isDatabaseReadGranted = ({
  readGrantedInternalIds,
  internalId,
}: {
  readGrantedInternalIds: Set<string>;
  internalId: string;
}) => {
  return readGrantedInternalIds.has(internalId);
};

const createDatabase = async ({
  databaseInternalId,
  usedInternalIds,
  allDatabases,
  connector,
}: {
  databaseInternalId: string;
  usedInternalIds: Set<string>;
  allDatabases: RemoteDatabaseModel[];
  connector: ConnectorResource;
}) => {
  // Mark as used
  usedInternalIds.add(databaseInternalId);

  // Check if the database exists
  const existingDb = allDatabases.find(
    (d) => d.internalId === databaseInternalId
  );

  if (!existingDb) {
    // Create and add the database to the list of all databases.
    allDatabases.push(
      await RemoteDatabaseModel.create({
        connectorId: connector.id,
        internalId: databaseInternalId,
        name: databaseInternalId,
        permission: "inherited",
      })
    );
  }

  // As we cannot know what is already in core, we'll create all databases in core after traversing the whole tree
};

const isSchemaReadGranted = ({
  readGrantedInternalIds,
  internalId,
}: {
  readGrantedInternalIds: Set<string>;
  internalId: string;
}) => {
  const { database_name } = parseSchemaInternalId(internalId);

  return (
    readGrantedInternalIds.has(database_name) ||
    readGrantedInternalIds.has(internalId)
  );
};

const createSchemaAndHierarchy = async ({
  schemaInternalId,
  usedInternalIds,
  allDatabases,
  allSchemas,
  connector,
}: {
  schemaInternalId: string;
  usedInternalIds: Set<string>;
  allDatabases: RemoteDatabaseModel[];
  allSchemas: RemoteSchemaModel[];
  connector: ConnectorResource;
}) => {
  const { database_name, name } = parseSchemaInternalId(schemaInternalId);

  await createDatabase({
    databaseInternalId: database_name,
    usedInternalIds,
    allDatabases,
    connector,
  });

  // Mark as used
  usedInternalIds.add(schemaInternalId);

  // Check if the schema exists
  const existingSchema = allSchemas.find(
    (s) => s.internalId === schemaInternalId
  );
  if (!existingSchema) {
    // Create and add the schema to the list of all schemas.
    allSchemas.push(
      await RemoteSchemaModel.create({
        connectorId: connector.id,
        internalId: schemaInternalId,
        name: name,
        databaseName: database_name,
        permission: "inherited",
      })
    );
  }
  // As we cannot know what is already in core, we'll create all schemas in core after traversing the whole tree
};

const isTableReadGranted = ({
  readGrantedInternalIds,
  internalId,
}: {
  readGrantedInternalIds: Set<string>;
  internalId: string;
}) => {
  const { database_name, schema_name } = parseTableInternalId(internalId);
  const schemaInternalId = [database_name, schema_name].join(".");

  return (
    readGrantedInternalIds.has(database_name) ||
    readGrantedInternalIds.has(schemaInternalId) ||
    readGrantedInternalIds.has(internalId)
  );
};

const createTableAndHierarchy = async ({
  tableInternalId,
  usedInternalIds,
  allTables,
  allSchemas,
  allDatabases,
  connector,
  mimeTypes,
}: {
  tableInternalId: string;
  usedInternalIds: Set<string>;
  allTables: RemoteTableModel[];
  allSchemas: RemoteSchemaModel[];
  allDatabases: RemoteDatabaseModel[];
  connector: ConnectorResource;
  mimeTypes: typeof MIME_TYPES.BIGQUERY | typeof MIME_TYPES.SNOWFLAKE;
}) => {
  const dataSourceConfig = dataSourceConfigFromConnector(connector);

  const {
    database_name: dbName,
    schema_name: schemaName,
    name: tableName,
  } = parseTableInternalId(tableInternalId);

  const schemaInternalId = [dbName, schemaName].join(".");

  await createSchemaAndHierarchy({
    schemaInternalId,
    usedInternalIds,
    allDatabases,
    allSchemas,
    connector,
  });

  // Mark as used
  usedInternalIds.add(tableInternalId);

  // Check it table already exists
  const existingTable = allTables.find((t) => t.internalId === tableInternalId);
  if (!existingTable) {
    // Create and add the table to the list of all tables.
    allTables.push(
      await RemoteTableModel.create({
        connectorId: connector.id,
        internalId: tableInternalId,
        name: tableName,
        schemaName,
        databaseName: dbName,
        permission: "inherited",
        lastUpsertedAt: new Date(),
      })
    );
  } else {
    await existingTable.update({
      lastUpsertedAt: new Date(),
    });
  }

  // ...upsert the table in core
  await upsertDataSourceRemoteTable({
    dataSourceConfig,
    tableId: tableInternalId,
    tableName: tableInternalId,
    remoteDatabaseTableId: tableInternalId,
    remoteDatabaseSecretId: connector.connectionId,
    tableDescription: "",
    parents: [tableInternalId, schemaInternalId, dbName],
    parentId: schemaInternalId,
    title: tableName,
    mimeType: mimeTypes.TABLE,
  });
};

export async function sync({
  remoteDBTree,
  connector,
  mimeTypes,
}: {
  remoteDBTree?: RemoteDBTree;
  connector: ConnectorResource;
  mimeTypes: typeof MIME_TYPES.BIGQUERY | typeof MIME_TYPES.SNOWFLAKE;
}) {
  const dataSourceConfig = dataSourceConfigFromConnector(connector);

  const [allDatabases, allSchemas, allTables] = await Promise.all([
    RemoteDatabaseModel.findAll({
      where: {
        connectorId: connector.id,
      },
    }),
    RemoteSchemaModel.findAll({
      where: {
        connectorId: connector.id,
      },
    }),
    RemoteTableModel.findAll({
      where: {
        connectorId: connector.id,
      },
    }),
  ]);

  const readGrantedInternalIds = new Set([
    ...allDatabases
      .filter((db) => db.permission === "selected")
      .map((db) => db.internalId),
    ...allSchemas
      .filter((s) => s.permission === "selected")
      .map((s) => s.internalId),
    ...allTables
      .filter((t) => t.permission === "selected")
      .map((t) => t.internalId),
  ]);

  const usedInternalIds = new Set<string>();

  logger.info(
    { connectorId: connector.id, databases: remoteDBTree?.databases },
    "Creating databases"
  );
  // Loop through the databases and create them if they are read granted
  for (const db of remoteDBTree?.databases ?? []) {
    if (
      isDatabaseReadGranted({
        readGrantedInternalIds,
        internalId: db.name,
      })
    ) {
      await createDatabase({
        databaseInternalId: db.name,
        usedInternalIds,
        allDatabases,
        connector,
      });
    }

    // Loop through the schemas and create them if they are read granted
    for (const schema of db.schemas) {
      const schemaInternalId = `${db.name}.${schema.name}`;
      if (
        isSchemaReadGranted({
          readGrantedInternalIds,
          internalId: schemaInternalId,
        })
      ) {
        await createSchemaAndHierarchy({
          schemaInternalId,
          usedInternalIds,
          allDatabases,
          allSchemas,
          connector,
        });
      }

      let i = 0;
      // Loop through the tables and create them if they are read granted
      for (const table of schema.tables) {
        const tableInternalId = `${table.database_name}.${table.schema_name}.${table.name}`;
        if (
          isTableReadGranted({
            readGrantedInternalIds,
            internalId: tableInternalId,
          })
        ) {
          await createTableAndHierarchy({
            tableInternalId,
            usedInternalIds,
            allTables,
            allSchemas,
            allDatabases,
            connector,
            mimeTypes,
          });
        }

        i++;
        if (i % 25 === 0) {
          await heartbeat();
        }
      }
    }
  }

  logger.info(
    {
      connectorId: connector.id,
      databasesToKeep: allDatabases.filter((db) =>
        usedInternalIds.has(db.internalId)
      ).length,
      schemasToKeep: allSchemas.filter((s) => usedInternalIds.has(s.internalId))
        .length,
      tablesToKeep: allTables.filter((t) => usedInternalIds.has(t.internalId))
        .length,
      databasesToRemove: allDatabases.filter(
        (db) => !usedInternalIds.has(db.internalId)
      ).length,
      schemasToRemove: allSchemas.filter(
        (s) => !usedInternalIds.has(s.internalId)
      ).length,
      tablesToRemove: allTables.filter(
        (t) => !usedInternalIds.has(t.internalId)
      ).length,
    },
    "Upserting databases, schemas and tables in core and removing unused ones"
  );

  for (const db of allDatabases) {
    if (usedInternalIds.has(db.internalId)) {
      await upsertDataSourceFolder({
        dataSourceConfig,
        folderId: db.internalId,
        title: db.name,
        parents: [db.internalId],
        parentId: null,
        mimeType: mimeTypes.DATABASE,
      });
    } else {
      await deleteDataSourceFolder({
        dataSourceConfig,
        folderId: db.internalId,
      });
      await db.destroy();

      if (db.permission === "selected") {
        logger.error(
          {
            connectorId: connector.id,
            databaseInternalId: db.internalId,
          },
          "Database is selected but not used, it should never happen and surface a flaw in the logic above."
        );
      }
    }
  }

  for (const schema of allSchemas) {
    if (usedInternalIds.has(schema.internalId)) {
      await upsertDataSourceFolder({
        dataSourceConfig,
        folderId: schema.internalId,
        title: schema.name,
        parents: [schema.internalId, schema.databaseName],
        parentId: schema.databaseName,
        mimeType: mimeTypes.SCHEMA,
      });
    } else {
      await deleteDataSourceFolder({
        dataSourceConfig,
        folderId: schema.internalId,
      });
      await schema.destroy();

      if (schema.permission === "selected") {
        logger.error(
          {
            connectorId: connector.id,
            schemaInternalId: schema.internalId,
          },
          "Schema is selected but not used, it should never happen and surface a flaw in the logic above."
        );
      }
    }
  }

  for (const unusedTable of allTables.filter(
    (table) => !usedInternalIds.has(table.internalId)
  )) {
    await deleteDataSourceTable({
      dataSourceConfig,
      tableId: unusedTable.internalId,
    });
    await unusedTable.destroy();

    if (unusedTable.permission === "selected") {
      logger.error(
        {
          connectorId: connector.id,
          tableInternalId: unusedTable.internalId,
        },
        "Table is selected but not used, it should never happen and surface a flaw in the logic above."
      );
    }
  }

  logger.info({ connectorId: connector.id }, "Sync completed");
}
