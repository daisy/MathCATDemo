import {
  ConditionalCheckFailedException,
  DynamoDBClient,
  UpdateItemCommand,
} from '@aws-sdk/client-dynamodb';
import { HttpError } from './errors.js';

const KEEP_SECONDS = 40 * 24 * 60 * 60;

let client;

// Approximates what engines bill: Google counts all SSML except <mark> tags.
export function billableCharacters(ssml) {
  return ssml.replace(/<mark\s[^>]*>/gi, '').length;
}

// Counts `characters` against today's (UTC) limit, or throws a 429 if they do not fit.
// The check and the update are one atomic write, so concurrent requests cannot overshoot.
// Any other failure also throws: without a working counter, nothing is synthesized.
export async function reserveCharacters(characters) {
  const table = process.env.USAGE_TABLE;
  const limit = Number.parseInt(process.env.DAILY_CHAR_LIMIT, 10);
  if (!table || !Number.isInteger(limit)) {
    throw new Error('USAGE_TABLE and DAILY_CHAR_LIMIT must be configured');
  }

  const room = limit - characters;
  if (room < 0) {
    throw dailyLimitError();
  }

  const now = new Date();
  client ??= new DynamoDBClient({});
  try {
    await client.send(
      new UpdateItemCommand({
        TableName: table,
        Key: { day: { S: now.toISOString().slice(0, 10) } },
        UpdateExpression: 'SET #expires = :expires ADD #used :characters',
        ConditionExpression: 'attribute_not_exists(#used) OR #used <= :room',
        ExpressionAttributeNames: { '#used': 'used', '#expires': 'expiresAt' },
        ExpressionAttributeValues: {
          ':characters': { N: String(characters) },
          ':room': { N: String(room) },
          ':expires': { N: String(Math.floor(now.getTime() / 1000) + KEEP_SECONDS) },
        },
      })
    );
  } catch (err) {
    if (err instanceof ConditionalCheckFailedException) {
      throw dailyLimitError();
    }
    throw err;
  }
}

function dailyLimitError() {
  console.warn('Daily character limit reached');
  return new HttpError(429, 'Daily speech limit reached');
}
