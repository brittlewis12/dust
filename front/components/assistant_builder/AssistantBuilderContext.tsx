import { createContext } from "react";

import type { MCPServerViewType } from "@app/lib/api/mcp";
import type { AppType, DataSourceViewType, SpaceType } from "@app/types";

type AssistantBuilderContextType = {
  dustApps: AppType[];
  dataSourceViews: DataSourceViewType[];
  spaces: SpaceType[];
  mcpServerViews: MCPServerViewType[];
};

export const AssistantBuilderContext =
  createContext<AssistantBuilderContextType>({
    dustApps: [],
    dataSourceViews: [],
    spaces: [],
    mcpServerViews: [],
  });

export function AssistantBuilderProvider({
  dustApps,
  dataSourceViews,
  spaces,
  mcpServerViews,
  children,
}: AssistantBuilderContextType & {
  children: React.ReactNode;
}) {
  return (
    <AssistantBuilderContext.Provider
      value={{
        dustApps,
        dataSourceViews,
        spaces,
        mcpServerViews,
      }}
    >
      {children}
    </AssistantBuilderContext.Provider>
  );
}
