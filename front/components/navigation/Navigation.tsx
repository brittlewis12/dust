import {
  Button,
  cn,
  MenuIcon,
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@dust-tt/sparkle";
import type { SubscriptionType, WorkspaceType } from "@dust-tt/types";
import { VisuallyHidden } from "@radix-ui/react-visually-hidden";
import { useRouter } from "next/router";
import React, { useContext, useMemo } from "react";

import type { SidebarNavigation } from "@app/components/navigation/config";
import {
  NavigationSidebar,
  ToggleNavigationSidebarButton,
} from "@app/components/navigation/NavigationSidebar";
import { SidebarContext } from "@app/components/sparkle/SidebarContext";
import WorkspacePicker from "@app/components/WorkspacePicker";
import { useUser } from "@app/lib/swr/user";
import { classNames } from "@app/lib/utils";

interface NavigationProps {
  hideSidebar: boolean;
  owner: WorkspaceType;
  subscription: SubscriptionType;
  navChildren?: React.ReactNode;
  subNavigation?: SidebarNavigation[] | null;
  isNavigationBarOpen: boolean;
  setNavigationBarOpen: (isOpen: boolean) => void;
}

export function Navigation({
  hideSidebar,
  owner,
  subscription,
  navChildren,
  subNavigation,
  isNavigationBarOpen,
  setNavigationBarOpen,
}: NavigationProps) {
  const { sidebarOpen, setSidebarOpen } = useContext(SidebarContext);

  const router = useRouter();
  const { user } = useUser();

  const workspacePicker = useMemo(() => {
    if (user && user.workspaces.length > 1) {
      return (
        <WorkspacePicker
          user={user}
          workspace={owner}
          onWorkspaceUpdate={async (workspace) => {
            const assistantRoute = `/w/${workspace.sId}/assistant/new`;
            if (workspace.id !== owner.id) {
              await router.push(assistantRoute).then(() => router.reload());
            }
          }}
        />
      );
    }
    return null;
  }, [owner, router, user]);

  if (hideSidebar) {
    return null;
  }

  return (
    <div
      className={cn(
        "flex shrink-0 overflow-x-hidden border-r",
        "border-border-dark dark:border-border-dark-night"
      )}
    >
      {/* Mobile sidebar */}
      <Sheet open={sidebarOpen} onOpenChange={setSidebarOpen}>
        <div className="fixed left-0 top-0 z-40 flex h-16 shrink-0 items-center gap-x-4 px-4 lg:hidden lg:px-6">
          <SheetTrigger asChild>
            <Button
              variant="ghost"
              icon={MenuIcon}
              onClick={() => setSidebarOpen(true)}
            />
          </SheetTrigger>
        </div>
        <SheetContent side="left" className="flex w-full max-w-xs flex-1">
          <SheetHeader className="bg-muted-background p-0">
            <VisuallyHidden>
              <SheetTitle className="hidden" />
            </VisuallyHidden>
            {workspacePicker}
          </SheetHeader>
          <NavigationSidebar
            subscription={subscription}
            owner={owner}
            subNavigation={subNavigation}
            user={user}
          >
            {navChildren && navChildren}
          </NavigationSidebar>
        </SheetContent>
      </Sheet>

      {/*Desktop sidebar*/}
      <div
        className={cn(
          "transition-width hidden flex-none overflow-hidden duration-150 ease-out lg:flex lg:flex-col",
          isNavigationBarOpen ? "w-80" : "w-0"
        )}
      >
        <div className="hidden flex-1 bg-muted-background dark:bg-muted-background-night lg:inset-y-0 lg:z-0 lg:flex lg:w-80 lg:flex-col">
          {workspacePicker}
          <NavigationSidebar
            owner={owner}
            subscription={subscription}
            subNavigation={subNavigation}
            user={user}
          >
            {navChildren && navChildren}
          </NavigationSidebar>
        </div>
      </div>

      <div
        className={classNames(
          "fixed z-40 hidden lg:top-1/2 lg:flex",
          isNavigationBarOpen ? "lg:ml-80" : ""
        )}
      >
        <ToggleNavigationSidebarButton
          isNavigationBarOpened={isNavigationBarOpen}
          toggleNavigationBarVisibility={(navigationBar) => {
            setNavigationBarOpen(navigationBar);
          }}
        />
      </div>
    </div>
  );
}
