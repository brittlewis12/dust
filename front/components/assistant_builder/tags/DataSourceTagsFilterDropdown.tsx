import {
  Button,
  Label,
  Page,
  PopoverContent,
  PopoverRoot,
  PopoverTrigger,
  SliderToggle,
} from "@dust-tt/sparkle";
import type {
  DataSourceTag,
  DataSourceViewSelectionConfiguration,
  DataSourceViewSelectionConfigurations,
  TagsFilter,
  WorkspaceType,
} from "@dust-tt/types";
import { useRef, useState } from "react";

import { getActionTags } from "@app/components/assistant_builder/tags/helpers";
import { TagSearchInput } from "@app/components/assistant_builder/tags/TagSearchInput";
import { useTagSearchEndpoint } from "@app/lib/swr/data_sources";
import { debounce } from "@app/lib/utils/debounce";

interface DataSourceTagsFilterDropdown {
  owner: WorkspaceType;
  dataSourceConfigurations: DataSourceViewSelectionConfigurations;
  currentDataSourceConfiguration: DataSourceViewSelectionConfiguration;
  onSave: (dsConfigs: DataSourceViewSelectionConfigurations) => void;
}

export function DataSourceTagsFilterDropdown({
  owner,
  dataSourceConfigurations,
  currentDataSourceConfiguration,
  onSave,
}: DataSourceTagsFilterDropdown) {
  const dataSource = currentDataSourceConfiguration.dataSourceView.dataSource;
  const selectedTagsIn = getActionTags(currentDataSourceConfiguration, "in");
  const selectedTagsNot = getActionTags(currentDataSourceConfiguration, "not");
  const dustAPIDataSourceIds = [dataSource.dustAPIDataSourceId];

  // State to save the tags filter before the dynamic filtering is enabled.
  // This is used to restore the tags filter when the dynamic filtering is disabled right after it was enabled.
  const [backUpTagsFilter, setBackUpTagsFilter] = useState<TagsFilter | null>(
    null
  );

  const {
    searchInputValueIn,
    searchInputValueNot,
    availableTagsIn,
    availableTagsNot,
    handleSearchInputChange,
    isLoadingIn,
    isLoadingNot,
  } = useTagSearch({
    owner,
    dustAPIDataSourceIds,
    selectedTagsIn,
    selectedTagsNot,
  });

  const handleTagOperation = (
    tag: DataSourceTag,
    mode: "in" | "not",
    operation: "add" | "remove"
  ) => {
    const newDsc = { ...currentDataSourceConfiguration };
    if (!newDsc.tagsFilter || newDsc.tagsFilter === "auto") {
      newDsc.tagsFilter = { in: [], not: [] };
    } else {
      // We need a Deep copy otherwise we will mutate the original object and we will not be able to cancel the changes if we cancel the modal.
      newDsc.tagsFilter = { ...newDsc.tagsFilter };
    }

    if (operation === "add") {
      newDsc.tagsFilter[mode] = [...newDsc.tagsFilter[mode], tag.tag];
    } else {
      newDsc.tagsFilter[mode] = newDsc.tagsFilter[mode].filter(
        (t: string) => t !== tag.tag
      );
    }

    onSave({
      ...dataSourceConfigurations,
      [newDsc.dataSourceView.sId]: newDsc,
    });
  };

  const handleAutoFilter = (isChecked: boolean) => {
    const newDsc = { ...currentDataSourceConfiguration };

    if (isChecked) {
      if (newDsc.tagsFilter && newDsc.tagsFilter !== "auto") {
        setBackUpTagsFilter({ ...newDsc.tagsFilter });
      }
      newDsc.tagsFilter = "auto";
    } else if (backUpTagsFilter) {
      newDsc.tagsFilter = backUpTagsFilter;
      setBackUpTagsFilter(null);
    }

    onSave({
      ...dataSourceConfigurations,
      [newDsc.dataSourceView.sId]: newDsc,
    });
  };

  const tagsFilter = currentDataSourceConfiguration.tagsFilter;
  let tagsLabels = "Filters";
  if (tagsFilter === "auto") {
    tagsLabels = "Filters (auto)";
  } else if (tagsFilter) {
    tagsLabels = `Filters (${tagsFilter.in.length + tagsFilter.not.length})`;
  }

  return (
    <PopoverRoot
      onOpenChange={(open) => {
        if (!open) {
          handleSearchInputChange("", "in");
          handleSearchInputChange("", "not");
        }
      }}
      modal={true}
    >
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" label={tagsLabels} isSelect />
      </PopoverTrigger>
      <PopoverContent className="w-[600px] max-w-[600px]">
        <div className="flex flex-col gap-8 p-2">
          <div className="flex flex-col gap-2">
            <Page.SectionHeader
              title="Manual fitering"
              description="Match content that has any of the must have labels, ignore anything that has any of the must not have labels."
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label>Must have labels</Label>
            <TagSearchInput
              searchInputValue={searchInputValueIn}
              setSearchInputValue={(value) =>
                handleSearchInputChange(value, "in")
              }
              availableTags={availableTagsIn}
              selectedTags={selectedTagsIn}
              onTagAdd={(tag) => handleTagOperation(tag, "in", "add")}
              onTagRemove={(tag) => handleTagOperation(tag, "in", "remove")}
              isLoading={isLoadingIn}
              disabled={tagsFilter === "auto"}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label>Must not have labels</Label>
            <TagSearchInput
              searchInputValue={searchInputValueNot}
              setSearchInputValue={(value) =>
                handleSearchInputChange(value, "not")
              }
              availableTags={availableTagsNot}
              selectedTags={selectedTagsNot}
              onTagAdd={(tag) => handleTagOperation(tag, "not", "add")}
              onTagRemove={(tag) => handleTagOperation(tag, "not", "remove")}
              tagChipColor="red"
              isLoading={isLoadingNot}
              disabled={tagsFilter === "auto"}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Page.SectionHeader
              title="Dynamic filtering"
              description="Allow the assistant to automatically determine relevant labels to include or exclude based on conversation context, overriding manual label filters."
            />
          </div>
          <div className="flex flex-row items-center gap-2">
            <SliderToggle
              selected={tagsFilter === "auto"}
              onClick={() => {
                handleAutoFilter(tagsFilter !== "auto");
              }}
              size="xs"
            />
            <Label>Enable dynamic filtering</Label>
          </div>
        </div>
      </PopoverContent>
    </PopoverRoot>
  );
}

/**
 * Handle the search input state logic.
 */
function useTagSearch({
  owner,
  dustAPIDataSourceIds,
  selectedTagsIn,
  selectedTagsNot,
}: {
  owner: WorkspaceType;
  dustAPIDataSourceIds: string[];
  selectedTagsIn: DataSourceTag[];
  selectedTagsNot: DataSourceTag[];
}) {
  const [searchInputValueIn, setSearchInputValueIn] = useState<string>("");
  const [searchInputValueNot, setSearchInputValueNot] = useState<string>("");
  const [availableTagsIn, setAvailableTagsIn] = useState<DataSourceTag[]>([]);
  const [availableTagsNot, setAvailableTagsNot] = useState<DataSourceTag[]>([]);
  const searchHandleIn = useRef<NodeJS.Timeout>();
  const searchHandleNot = useRef<NodeJS.Timeout>();
  const [isLoadingIn, setIsLoadingIn] = useState(false);
  const [isLoadingNot, setIsLoadingNot] = useState(false);
  const { searchTags } = useTagSearchEndpoint({ owner });

  const searchTagsInCoreAPI = async (query: string) => {
    try {
      const tags = await searchTags({
        query,
        queryType: "prefix",
        dataSources: dustAPIDataSourceIds,
      });
      const formattedTags: DataSourceTag[] = [];
      for (const tag of tags) {
        for (const dataSourceId of tag.data_sources) {
          const isTagUsed =
            selectedTagsIn.some(
              (t) => t.tag === tag.tag && t.dustAPIDataSourceId === dataSourceId
            ) ||
            selectedTagsNot.some(
              (t) => t.tag === tag.tag && t.dustAPIDataSourceId === dataSourceId
            );

          if (!isTagUsed) {
            formattedTags.push({
              tag: tag.tag,
              dustAPIDataSourceId: dataSourceId,
              connectorProvider: null,
            });
          }
        }
      }
      return formattedTags;
    } catch (error) {
      console.error("Failed to search tags:", error);
      return [];
    }
  };

  const handleSearchInputChange = (value: string, mode: "in" | "not") => {
    const setSearchValue =
      mode === "in" ? setSearchInputValueIn : setSearchInputValueNot;
    const setAvailableTags =
      mode === "in" ? setAvailableTagsIn : setAvailableTagsNot;
    const setIsLoading = mode === "in" ? setIsLoadingIn : setIsLoadingNot;
    const searchHandle = mode === "in" ? searchHandleIn : searchHandleNot;

    setSearchValue(value);

    if (!value.trim()) {
      setAvailableTags([]);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    debounce(
      searchHandle,
      async () => {
        try {
          const formattedTags = await searchTagsInCoreAPI(value);
          setAvailableTags(formattedTags);
        } catch (error) {
          console.error("Failed to search tags:", error);
          setAvailableTags([]);
        } finally {
          setIsLoading(false);
        }
      },
      500
    );
  };

  return {
    searchInputValueIn,
    searchInputValueNot,
    availableTagsIn,
    availableTagsNot,
    handleSearchInputChange,
    isLoadingIn,
    isLoadingNot,
  };
}
