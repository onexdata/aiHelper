import jsonata from "jsonata";
import { invoke } from "@tauri-apps/api/core";

let tagInterval = null;
const BATCH_SIZE = 200;
const TAG_INTERVAL_MS = 10000;

export function startTagger() {
  if (!tagInterval) {
    runTagger();
    tagInterval = setInterval(runTagger, TAG_INTERVAL_MS);
  }
}

export function stopTagger() {
  if (tagInterval) {
    clearInterval(tagInterval);
    tagInterval = null;
  }
}

/**
 * Run one tagging cycle. Optional `onProgress` callback receives status objects:
 *   { phase: "compiling", ruleCount }
 *   { phase: "batch", batchNum, batchSize, tagged, totalTagged, skipped, totalSkipped }
 *   { phase: "done", totalTagged, totalSkipped }
 *   { phase: "idle" }            — nothing to do (no rules or no activity)
 *   { phase: "error", message }
 */
export async function runTagger(onProgress) {
  const report = onProgress || (() => {});
  try {
    const rules = await invoke("get_all_rules");
    if (rules.length === 0) {
      report({ phase: "idle" });
      return;
    }

    // Compile JSONata expressions once per cycle
    const compiled = [];
    for (const r of rules) {
      try {
        compiled.push({
          project_id: r.project_id,
          expr: jsonata(r.expression),
        });
      } catch (e) {
        console.warn(`Invalid JSONata expression (rule ${r.id}):`, e.message);
      }
    }
    if (compiled.length === 0) {
      report({ phase: "idle" });
      return;
    }

    report({ phase: "compiling", ruleCount: compiled.length });

    let totalTagged = 0;
    let totalSkipped = 0;
    let batchNum = 0;

    let batch = await invoke("get_untagged_activity", { limit: BATCH_SIZE });
    while (batch.length > 0) {
      batchNum++;
      const tags = [];
      let skipped = 0;
      for (const row of batch) {
        let matched = false;
        for (const rule of compiled) {
          try {
            const result = await rule.expr.evaluate(row);
            if (result) {
              tags.push({ table: row.table, id: row.id, project_id: rule.project_id });
              matched = true;
              break; // first match wins
            }
          } catch {
            // expression error on this row, skip
          }
        }
        if (!matched) skipped++;
      }
      if (tags.length > 0) {
        await invoke("tag_activities", { tags });
      }
      totalTagged += tags.length;
      totalSkipped += skipped;
      report({
        phase: "batch",
        batchNum,
        batchSize: batch.length,
        tagged: tags.length,
        totalTagged,
        skipped,
        totalSkipped,
      });
      if (tags.length === 0) {
        // No matches in this batch — remaining rows don't match any rule
        break;
      }
      batch = await invoke("get_untagged_activity", { limit: BATCH_SIZE });
    }

    report({ phase: "done", totalTagged, totalSkipped });
  } catch (e) {
    console.error("Tagger error:", e);
    report({ phase: "error", message: String(e) });
  }
}
