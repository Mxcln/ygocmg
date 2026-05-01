import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { standardPackApi } from "../../shared/api/standardPackApi";
import { stringsApi } from "../../shared/api/stringsApi";
import { mergeSetnameEntries, type SetnameEntry } from "./setnameEntries";

interface UseMergedSetnameEntriesInput {
  workspaceId: string | null | undefined;
  packId: string | null | undefined;
  language: string | null | undefined;
  standardLanguage: string | null | undefined;
  enabled?: boolean;
}

export function useMergedSetnameEntries({
  workspaceId,
  packId,
  language,
  standardLanguage,
  enabled = true,
}: UseMergedSetnameEntriesInput) {
  const packSetnamesQuery = useQuery({
    queryKey: ["pack-setnames", packId, language],
    queryFn: () =>
      stringsApi.listPackStrings({
        workspaceId: workspaceId!,
        packId: packId!,
        language: language!,
        kindFilter: "setname",
        keyword: null,
        keyFilter: null,
        page: 1,
        pageSize: 10000,
      }),
    enabled: enabled && Boolean(workspaceId && packId && language),
    staleTime: 30_000,
  });

  const standardSetnamesQuery = useQuery({
    queryKey: ["standard-setnames", standardLanguage],
    queryFn: () => standardPackApi.listSetnames({ language: standardLanguage ?? null }),
    enabled: enabled && Boolean(standardLanguage),
    staleTime: 5 * 60 * 1000,
  });

  const setnameEntries = useMemo<SetnameEntry[]>(() => {
    const packEntries =
      packSetnamesQuery.data?.items.map((item) => ({
        key: item.key,
        name: item.value,
        source: "pack" as const,
      })) ?? [];
    const standardEntries =
      standardSetnamesQuery.data?.map((item) => ({
        key: item.key,
        name: item.value,
        source: "standard" as const,
      })) ?? [];
    return mergeSetnameEntries(packEntries, standardEntries);
  }, [packSetnamesQuery.data, standardSetnamesQuery.data]);

  return {
    setnameEntries,
    packSetnamesQuery,
    standardSetnamesQuery,
  };
}
