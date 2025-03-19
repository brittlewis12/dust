import { CONNECTOR_CONFIGURATIONS } from "@app/shared/lib/connector_providers";
import type {
  ContentNodeType,
  DataSourceViewContentNodeType,
} from "@dust-tt/client";
import { MIME_TYPES } from "@dust-tt/client";
import {
  assertNever,
  ChatBubbleLeftRightIcon,
  DocumentIcon,
  DocumentPileIcon,
  FolderIcon,
  FolderTableIcon,
  LockIcon,
  Square3Stack3DIcon,
} from "@dust-tt/sparkle";

// Since titles will be synced in ES we don't support arbitrarily large titles.
export const MAX_NODE_TITLE_LENGTH = 512;

// Mime types that should be represented with a Channel icon.
export const CHANNEL_MIME_TYPES = [
  MIME_TYPES.GITHUB.DISCUSSIONS,
  MIME_TYPES.INTERCOM.TEAM,
  MIME_TYPES.INTERCOM.TEAMS_FOLDER,
  MIME_TYPES.SLACK.CHANNEL,
] as readonly string[];

// Mime types that should be represented with a Database icon but are not of type "table".
export const DATABASE_MIME_TYPES = [
  MIME_TYPES.GITHUB.ISSUES,
] as readonly string[];

// Mime types that should be represented with a File icon but are not of type "document".
export const FILE_MIME_TYPES = [
  MIME_TYPES.WEBCRAWLER.FOLDER,
] as readonly string[];

// Mime types that should be represented with a Spreadsheet icon, despite being of type "folder".
export const SPREADSHEET_MIME_TYPES = [
  MIME_TYPES.GOOGLE_DRIVE.SPREADSHEET,
  MIME_TYPES.MICROSOFT.SPREADSHEET,
] as readonly string[];

// Mime type that represents a datasource.
export const DATA_SOURCE_MIME_TYPE = "application/vnd.dust.datasource";

function getVisualForFileContentNode(
  node: ContentNodeType & { type: "document" }
) {
  if (node.expandable) {
    return DocumentPileIcon;
  }

  return DocumentIcon;
}

export function getVisualForDataSourceViewContentNode(
  node: DataSourceViewContentNodeType
) {
  // Handle data sources with connector providers.
  if (
    node.mimeType &&
    node.mimeType === DATA_SOURCE_MIME_TYPE &&
    node.dataSourceView?.dataSource?.connectorProvider &&
    CONNECTOR_CONFIGURATIONS[node.dataSourceView.dataSource.connectorProvider]
  ) {
    return CONNECTOR_CONFIGURATIONS[
      node.dataSourceView.dataSource.connectorProvider
    ].getLogoComponent();
  }

  // Fall back to regular content node icon handling.
  return getVisualForContentNode(node);
}

export function getVisualForContentNode(node: ContentNodeType) {
  // Check mime type first for special icon handling.
  if (node.mimeType) {
    // Handle private channels with lock icon.
    if (CHANNEL_MIME_TYPES.includes(node.mimeType)) {
      return node.providerVisibility === "private"
        ? LockIcon
        : ChatBubbleLeftRightIcon;
    }

    // Handle database-like content.
    if (DATABASE_MIME_TYPES.includes(node.mimeType)) {
      return Square3Stack3DIcon;
    }

    // Handle file-like content that isn't a document type.
    if (FILE_MIME_TYPES.includes(node.mimeType)) {
      return getVisualForFileContentNode(
        node as ContentNodeType & { type: "document" }
      );
    }

    // Handle spreadsheets.
    if (SPREADSHEET_MIME_TYPES.includes(node.mimeType)) {
      return FolderTableIcon;
    }
  }

  // Fall back to node type if mime type doesn't determine the icon.
  switch (node.type) {
    case "table":
      return Square3Stack3DIcon;

    case "folder":
      return FolderIcon;

    case "document":
      return getVisualForFileContentNode(
        node as ContentNodeType & { type: "document" }
      );

    default:
      assertNever(node.type);
  }
}

export function getLocationForDataSourceViewContentNode(
  node: DataSourceViewContentNodeType
) {
  const { dataSource } = node.dataSourceView;
  const { connectorProvider } = dataSource;
  const providerName = connectorProvider
    ? CONNECTOR_CONFIGURATIONS[connectorProvider].name
    : "Folders";

  if (!node.parentTitle) {
    return providerName;
  }

  return `${providerName} › ... › ${node.parentTitle}`;
}
