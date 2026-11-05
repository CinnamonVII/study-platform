// Small CI helper; the application and its stateful pipeline remain Rust.
const { appendFileSync } = require('node:fs');

async function checkPages(env, request = fetch) {
  if (!env.NOTION_TOKEN || !env.NOTION_CHECK_PAGE_IDS) {
    return { status: 'not_configured', checked: 0, failed: 0 };
  }
  let ids;
  try {
    ids = JSON.parse(env.NOTION_CHECK_PAGE_IDS);
  } catch {
    throw new Error('NOTION_CHECK_PAGE_IDS must be a JSON array of page IDs.');
  }
  if (!Array.isArray(ids) || ids.length === 0 || ids.some(id =>
    typeof id !== 'string' || !/^(?:[a-f0-9]{32}|[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.test(id))) {
    throw new Error('NOTION_CHECK_PAGE_IDS must contain valid page IDs only.');
  }
  ids = [...new Set(ids.map(id => id.replaceAll('-', '').toLowerCase()))];
  let failed = 0;
  for (const id of ids) {
    try {
      const response = await request(`https://api.notion.com/v1/pages/${id}`, {
        method: 'GET',
        headers: { Authorization: `Bearer ${env.NOTION_TOKEN}`, 'Notion-Version': '2025-09-03' },
        signal: AbortSignal.timeout(20000),
        redirect: 'error',
      });
      if (!response.ok) {
        failed++;
      } else {
        const page = await response.json();
        if (page.object !== 'page' || page.archived || page.in_trash) failed++;
      }
    } catch {
      // Never print response bodies, URLs, tokens, or network exception text.
      failed++;
    }
    // Stay below Notion's average request limit without maintaining a queue.
    await new Promise(resolve => setTimeout(resolve, 350));
  }
  return { status: failed ? 'failed' : 'checked', checked: ids.length, failed };
}

async function main() {
  try {
    const result = await checkPages(process.env);
    const report = result.status === 'not_configured'
      ? 'NOT CONFIGURED: set repository secrets NOTION_TOKEN and NOTION_CHECK_PAGE_IDS. No Notion access was checked.'
      : `Notion metadata access: ${result.checked} page(s) checked, ${result.failed} failed.`;
    const summary = `${report}\n\nRead-only metadata check. No content sync, change tracking, generation, exports, or publication. No persistent storage.\n`;
    console.log(summary);
    if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
    if (result.status === 'failed') process.exitCode = 1;
  } catch (error) {
    // Only our configuration errors can reach this handler.
    console.error(error.message);
    process.exitCode = 1;
  }
}

module.exports = { checkPages };
if (require.main === module) main();
