export interface SetnameEntry {
  key: number;
  name: string;
  source: "pack" | "standard";
}

export function mergeSetnameEntries(
  packEntries: SetnameEntry[],
  standardEntries: SetnameEntry[],
): SetnameEntry[] {
  const byKey = new Map<number, SetnameEntry>();
  for (const entry of packEntries) {
    byKey.set(entry.key, entry);
  }
  for (const entry of standardEntries) {
    if (!byKey.has(entry.key)) {
      byKey.set(entry.key, entry);
    }
  }
  return [...byKey.values()];
}

export function sortSetnameEntries(entries: SetnameEntry[]): SetnameEntry[] {
  return [...entries].sort((left, right) => {
    if (left.source !== right.source) return left.source === "pack" ? -1 : 1;
    return left.name.localeCompare(right.name);
  });
}
