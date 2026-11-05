const { test } = require('node:test');
const assert = require('node:assert/strict');
const { checkPages } = require('./notion-check.cjs');
const id = 'a'.repeat(32);
const env = { NOTION_TOKEN: 'test-only', NOTION_CHECK_PAGE_IDS: JSON.stringify([id]) };

test('missing credentials do not claim successful access or send requests', async () => {
  const result = await checkPages({}, () => assert.fail('must not request'));
  assert.equal(result.status, 'not_configured');
  assert.equal(result.checked, 0);
});
test('invalid destinations fail before sending the credential', async () => {
  for (const value of ['{', '[]', '["https://example.com"]', 'null']) {
    await assert.rejects(checkPages({ ...env, NOTION_CHECK_PAGE_IDS: value }, () => assert.fail('must not request')));
  }
});
test('only explicit deduplicated page metadata is read', async () => {
  let calls = 0;
  const result = await checkPages({ ...env, NOTION_CHECK_PAGE_IDS: JSON.stringify([id, id.toUpperCase()]) }, async (url, options) => {
    calls++;
    assert.equal(url, `https://api.notion.com/v1/pages/${id}`);
    assert.equal(options.method, 'GET');
    assert.equal(options.redirect, 'error');
    return { ok: true, json: async () => ({ object: 'page', archived: false }) };
  });
  assert.equal(calls, 1);
  assert.equal(result.status, 'checked');
});
test('partial access and trashed pages fail without discarding other checks', async () => {
  let calls = 0;
  const result = await checkPages({ ...env, NOTION_CHECK_PAGE_IDS: JSON.stringify([id, 'b'.repeat(32), 'c'.repeat(32)]) }, async () => {
    calls++;
    if (calls === 1) return { ok: false };
    return { ok: true, json: async () => ({ object: 'page', in_trash: calls === 2 }) };
  });
  assert.deepEqual(result, { status: 'failed', checked: 3, failed: 2 });
});
test('network errors stay redacted', async () => {
  const result = await checkPages(env, async () => { throw new Error('sensitive URL or token'); });
  assert.deepEqual(result, { status: 'failed', checked: 1, failed: 1 });
});
