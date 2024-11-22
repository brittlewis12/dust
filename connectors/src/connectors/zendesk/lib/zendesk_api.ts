import assert from "node:assert";

import type { ModelId } from "@dust-tt/types";
import type { Client } from "node-zendesk";
import { createClient } from "node-zendesk";

import type {
  ZendeskFetchedArticle,
  ZendeskFetchedCategory,
  ZendeskFetchedTicket,
} from "@connectors/@types/node-zendesk";
import { ExternalOAuthTokenError } from "@connectors/lib/error";
import logger from "@connectors/logger/logger";
import { ZendeskBrandResource } from "@connectors/resources/zendesk_resources";

const ZENDESK_RATE_LIMIT_MAX_RETRIES = 5;
const ZENDESK_RATE_LIMIT_TIMEOUT_SECONDS = 60;

/**
 * Retrieves the endpoint part from a URL used to call the Zendesk API.
 */
function getEndpointFromUrl(url: string): string {
  return url.split("api/v2")[1] as string;
}

export function createZendeskClient({
  accessToken,
  subdomain,
}: {
  accessToken: string;
  subdomain: string;
}) {
  return createClient({ oauth: true, token: accessToken, subdomain });
}

/**
 * Returns a Zendesk client with the subdomain set to the one in the brand.
 * Retrieves the brand from the database if it exists, fetches it from the Zendesk API otherwise.
 * @returns The subdomain of the brand the client was scoped to.
 */
export async function changeZendeskClientSubdomain(
  client: Client,
  { connectorId, brandId }: { connectorId: ModelId; brandId: number }
): Promise<string> {
  const brandSubdomain = await getZendeskBrandSubdomain(client, {
    connectorId,
    brandId,
  });
  client.config.subdomain = brandSubdomain;
  return brandSubdomain;
}

/**
 * Retrieves a brand's subdomain from the database if it exists, fetches it from the Zendesk API otherwise.
 */
async function getZendeskBrandSubdomain(
  client: Client,
  { connectorId, brandId }: { connectorId: ModelId; brandId: number }
): Promise<string> {
  const brandInDb = await ZendeskBrandResource.fetchByBrandId({
    connectorId,
    brandId,
  });
  if (brandInDb) {
    return brandInDb.subdomain;
  }

  const {
    result: { brand },
  } = await client.brand.show(brandId);
  return brand.subdomain;
}

/**
 * Handles rate limit responses from Zendesk API.
 * Expects to find the header `Retry-After` in the response.
 * https://developer.zendesk.com/api-reference/introduction/rate-limits/
 * @returns true if the rate limit was handled and the request should be retried, false otherwise.
 */
async function handleZendeskRateLimit(response: Response): Promise<boolean> {
  if (response.status === 429) {
    const retryAfter = Math.max(
      Number(response.headers.get("Retry-After")) || 1,
      1
    );
    if (retryAfter > ZENDESK_RATE_LIMIT_TIMEOUT_SECONDS) {
      logger.info(
        { retryAfter },
        `[Zendesk] Attempting to wait more than ${ZENDESK_RATE_LIMIT_TIMEOUT_SECONDS} s, aborting.`
      );
      throw new Error(
        `Zendesk retry after larger than ${ZENDESK_RATE_LIMIT_TIMEOUT_SECONDS} s, aborting.`
      );
    }
    logger.info(
      { response, retryAfter },
      "[Zendesk] Rate limit hit, waiting before retrying."
    );
    await new Promise((resolve) => setTimeout(resolve, retryAfter * 1000));
    return true;
  }
  return false;
}

/**
 * Runs a GET request to the Zendesk API with a maximum number of retries before throwing.
 */
async function fetchFromZendeskWithRetries({
  url,
  accessToken,
}: {
  url: string;
  accessToken: string;
}) {
  const runFetch = async () =>
    fetch(url, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
    });

  let rawResponse = await runFetch();

  let retryCount = 0;
  while (await handleZendeskRateLimit(rawResponse)) {
    rawResponse = await runFetch();
    retryCount++;
    if (retryCount >= ZENDESK_RATE_LIMIT_MAX_RETRIES) {
      logger.info(
        { response: rawResponse },
        `[Zendesk] Rate limit hit more than ${ZENDESK_RATE_LIMIT_MAX_RETRIES}, aborting.`
      );
      throw new Error(
        `Zendesk rate limit hit more than ${ZENDESK_RATE_LIMIT_MAX_RETRIES} times, aborting.`
      );
    }
  }
  let response;
  try {
    response = await rawResponse.json();
  } catch (e) {
    if (rawResponse.status === 404) {
      logger.error(
        { rawResponse, text: rawResponse.text },
        `[Zendesk] Zendesk API 404 error on: ${getEndpointFromUrl(url)}`
      );
      return null;
    }
    logger.error(
      { rawResponse, status: rawResponse.status, text: rawResponse.text },
      "[Zendesk] Error parsing Zendesk API response"
    );
    throw new Error("Error parsing Zendesk API response");
  }
  if (!rawResponse.ok) {
    if (response.type === "error.list" && response.errors?.length) {
      const error = response.errors[0];
      if (error.code === "unauthorized") {
        throw new ExternalOAuthTokenError();
      }
      if (error.code === "not_found") {
        return null;
      }
    }
    logger.error(
      {
        rawResponse,
        response,
        status: rawResponse.status,
        endpoint: getEndpointFromUrl(url),
      },
      "[Zendesk] Zendesk API error"
    );
    throw new Error("Zendesk API error.");
  }

  return response;
}

/**
 * Fetches a batch of categories from the Zendesk API.
 */
export async function fetchZendeskCategoriesInBrand({
  brandSubdomain,
  accessToken,
  pageSize,
  cursor = null,
}: {
  brandSubdomain: string;
  accessToken: string;
  pageSize: number;
  cursor: string | null;
}): Promise<{
  categories: ZendeskFetchedCategory[];
  meta: { has_more: boolean; after_cursor: string };
}> {
  assert(
    pageSize <= 100,
    `pageSize must be at most 100 (current value: ${pageSize})` // https://developer.zendesk.com/api-reference/introduction/pagination
  );

  const response = await fetchFromZendeskWithRetries({
    url:
      `https://${brandSubdomain}.zendesk.com/api/v2/help_center/categories?page[size]=${pageSize}` +
      (cursor ? `&page[after]=${encodeURIComponent(cursor)}` : ""),
    accessToken,
  });
  return (
    response || { categories: [], meta: { has_more: false, after_cursor: "" } }
  );
}

/**
 * Fetches a batch of the recently updated articles from the Zendesk API using the incremental API endpoint.
 */
export async function fetchRecentlyUpdatedArticles({
  brandSubdomain,
  accessToken,
  startTime, // start time in Unix epoch time, in seconds
}: {
  brandSubdomain: string;
  accessToken: string;
  startTime: number;
}): Promise<{
  articles: ZendeskFetchedArticle[];
  next_page: string | null;
  end_time: number;
}> {
  // this endpoint retrieves changes in content despite what is mentioned in the documentation.
  const response = await fetchFromZendeskWithRetries({
    url: `https://${brandSubdomain}.zendesk.com/api/v2/help_center/incremental/articles.json?start_time=${startTime}`,
    accessToken,
  });
  return (
    response || {
      articles: [],
      next_page: null,
      end_time: startTime,
    }
  );
}

/**
 * Fetches a batch of articles in a category from the Zendesk API.
 */
export async function fetchZendeskArticlesInCategory({
  brandSubdomain,
  accessToken,
  categoryId,
  pageSize,
  cursor = null,
}: {
  brandSubdomain: string;
  accessToken: string;
  categoryId: number;
  pageSize: number;
  cursor: string | null;
}): Promise<{
  articles: ZendeskFetchedArticle[];
  meta: { has_more: boolean; after_cursor: string };
}> {
  assert(
    pageSize <= 100,
    `pageSize must be at most 100 (current value: ${pageSize})` // https://developer.zendesk.com/api-reference/introduction/pagination
  );

  const response = await fetchFromZendeskWithRetries({
    url:
      `https://${brandSubdomain}.zendesk.com/api/v2/help_center/categories/${categoryId}/articles?page[size]=${pageSize}` +
      (cursor ? `&page[after]=${encodeURIComponent(cursor)}` : ""),
    accessToken,
  });
  return (
    response || { articles: [], meta: { has_more: false, after_cursor: "" } }
  );
}

/**
 * Fetches a batch of the recently updated tickets from the Zendesk API using the incremental API endpoint.
 */
export async function fetchRecentlyUpdatedTickets({
  brandSubdomain,
  accessToken,
  startTime = null,
  cursor = null,
}: // pass either a cursor or a start time, but not both
| {
      brandSubdomain: string;
      accessToken: string;
      startTime: number | null;
      cursor?: never;
    }
  | {
      brandSubdomain: string;
      accessToken: string;
      startTime?: never;
      cursor: string | null;
    }): Promise<{
  tickets: ZendeskFetchedTicket[];
  end_of_stream: boolean;
  after_cursor: string;
}> {
  const response = await fetchFromZendeskWithRetries({
    url:
      `https://${brandSubdomain}.zendesk.com/api/v2/incremental/tickets/cursor.json` +
      (cursor ? `?cursor=${encodeURIComponent(cursor)}` : "") +
      (startTime ? `?start_time=${startTime}` : ""),
    accessToken,
  });
  return (
    response || {
      tickets: [],
      end_of_stream: false,
      after_cursor: "",
    }
  );
}

/**
 * Fetches a batch of tickets from the Zendesk API.
 * Only fetches tickets that have been solved, and that were updated within the retention period.
 */
export async function fetchZendeskTicketsInBrand({
  brandSubdomain,
  accessToken,
  pageSize,
  cursor,
  retentionPeriodDays,
}: {
  brandSubdomain: string;
  accessToken: string;
  pageSize: number;
  cursor: string | null;
  retentionPeriodDays: number;
}): Promise<{
  tickets: ZendeskFetchedTicket[];
  meta: { has_more: boolean; after_cursor: string };
}> {
  assert(
    pageSize <= 100,
    `pageSize must be at most 100 (current value: ${pageSize})`
  );

  const searchQuery = encodeURIComponent(
    `status:solved updated>${retentionPeriodDays}days`
  );
  const response = await fetchFromZendeskWithRetries({
    url:
      `https://${brandSubdomain}.zendesk.com/api/v2/search/export.json?query=${searchQuery}&filter[type]=ticket&page[size]=${pageSize}` +
      (cursor ? `&page[after]=${encodeURIComponent(cursor)}` : ""),
    accessToken,
  });

  return response
    ? {
        tickets: response.results || [],
        meta: {
          has_more: !!response.meta?.has_more,
          after_cursor: response.meta?.after_cursor || "",
        },
      }
    : { tickets: [], meta: { has_more: false, after_cursor: "" } };
}
