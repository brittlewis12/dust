import {
  BarChartIcon,
  Button,
  ChatBubbleBottomCenterTextIcon,
  classNames,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  LightbulbIcon,
  MagicIcon,
  Markdown,
  MoreIcon,
  Page,
  Spinner,
  Tabs,
  TabsList,
  TabsTrigger,
  XMarkIcon,
} from "@dust-tt/sparkle";
import type {
  AssistantBuilderRightPanelStatus,
  AssistantBuilderRightPanelTab,
  WorkspaceType,
} from "@dust-tt/types";
import { Separator } from "@radix-ui/react-select";
import { useContext, useEffect } from "react";

import ConversationViewer from "@app/components/assistant/conversation/ConversationViewer";
import { GenerationContextProvider } from "@app/components/assistant/conversation/GenerationContextProvider";
import { AssistantInputBar } from "@app/components/assistant/conversation/input_bar/InputBar";
import { FeedbacksSection } from "@app/components/assistant_builder/FeedbacksSection";
import {
  usePreviewAssistant,
  useTryAssistantCore,
} from "@app/components/assistant_builder/TryAssistant";
import type {
  AssistantBuilderSetActionType,
  AssistantBuilderState,
  BuilderScreen,
  TemplateActionType,
} from "@app/components/assistant_builder/types";
import { getDefaultActionConfiguration } from "@app/components/assistant_builder/types";
import { ConfirmContext } from "@app/components/Confirm";
import { ACTION_SPECIFICATIONS } from "@app/lib/api/assistant/actions/utils";
import { useUser } from "@app/lib/swr/user";
import type { FetchAssistantTemplateResponse } from "@app/pages/api/w/[wId]/assistant/builder/templates/[tId]";

interface AssistantBuilderRightPanelProps {
  screen: BuilderScreen;
  template: FetchAssistantTemplateResponse | null;
  removeTemplate: () => Promise<void>;
  resetToTemplateInstructions: () => Promise<void>;
  resetToTemplateActions: () => Promise<void>;
  owner: WorkspaceType;
  rightPanelStatus: AssistantBuilderRightPanelStatus;
  openRightPanelTab: (tabName: AssistantBuilderRightPanelTab) => void;
  builderState: AssistantBuilderState;
  agentConfigurationId: string | null;
  setAction: (action: AssistantBuilderSetActionType) => void;
}

export default function AssistantBuilderRightPanel({
  screen,
  template,
  removeTemplate,
  resetToTemplateInstructions,
  resetToTemplateActions,
  owner,
  rightPanelStatus,
  openRightPanelTab,
  builderState,
  agentConfigurationId,
  setAction,
}: AssistantBuilderRightPanelProps) {
  const {
    shouldAnimate: shouldAnimatePreviewDrawer,
    draftAssistant,
    isFading,
  } = usePreviewAssistant({
    owner,
    builderState,
    isPreviewOpened: rightPanelStatus.tab === "Preview",
  });

  const { user } = useUser();
  const {
    conversation,
    setConversation,
    stickyMentions,
    setStickyMentions,
    handleSubmit,
  } = useTryAssistantCore({
    owner,
    user,
    assistant: draftAssistant,
  });

  useEffect(() => {
    setConversation(null);
  }, [draftAssistant?.sId, setConversation]);

  useEffect(() => {
    if (rightPanelStatus.tab === "Template" && screen === "naming") {
      openRightPanelTab("Preview");
    }
  }, [screen, rightPanelStatus.tab, openRightPanelTab]);

  return (
    <div className="flex h-full flex-col">
      <div className="shrink-0 bg-white pt-5">
        <Tabs
          value={rightPanelStatus.tab ?? "Preview"}
          onValueChange={(t) =>
            openRightPanelTab(t as AssistantBuilderRightPanelTab)
          }
          className="hidden lg:flex"
        >
          <TabsList className="inline-flex items-center gap-2 border-b border-separator">
            {template && (
              <TabsTrigger value="Template" label="Template" icon={MagicIcon} />
            )}
            {/* The agentConfigurationId is truthy iff not a new assistant */}
            {agentConfigurationId && (
              <TabsTrigger
                value="Performance"
                label="Performance"
                icon={BarChartIcon}
              />
            )}
            <TabsTrigger
              value="Preview"
              label="Preview"
              icon={ChatBubbleBottomCenterTextIcon}
            />
          </TabsList>
        </Tabs>
      </div>
      <div
        className={classNames(
          template !== null
            ? "grow-1 mb-5 h-full overflow-y-auto rounded-b-xl border-x border-b border-structure-200 pt-5"
            : "grow-1 mb-5 mt-5 h-full overflow-y-auto rounded-xl border border-structure-200",
          shouldAnimatePreviewDrawer &&
            rightPanelStatus.tab === "Preview" &&
            rightPanelStatus.openedAt != null &&
            // Only animate the reload if the drawer has been open for at least 1 second.
            // This is to prevent the animation from triggering right after the drawer is opened.
            Date.now() - rightPanelStatus.openedAt > 1000
            ? "animate-reload"
            : "",
          rightPanelStatus.tab !== "Performance" ? "bg-structure-50" : ""
        )}
      >
        {(rightPanelStatus.tab === "Preview" || screen === "naming") &&
          user && (
            <div className="flex h-full w-full flex-1 flex-col justify-between overflow-x-hidden">
              {draftAssistant ? (
                <GenerationContextProvider>
                  <div className="flex-grow overflow-y-auto">
                    {conversation && (
                      <ConversationViewer
                        owner={owner}
                        user={user}
                        conversationId={conversation.sId}
                        onStickyMentionsChange={setStickyMentions}
                        isInModal
                        isFading={isFading}
                        key={conversation.sId}
                      />
                    )}
                  </div>
                  <div className="shrink-0">
                    <AssistantInputBar
                      owner={owner}
                      onSubmit={handleSubmit}
                      stickyMentions={stickyMentions}
                      conversationId={conversation?.sId || null}
                      additionalAgentConfiguration={draftAssistant}
                      actions={["attachment"]}
                      disableAutoFocus
                      isFloating={false}
                    />
                  </div>
                </GenerationContextProvider>
              ) : (
                <div className="flex h-full w-full items-center justify-center">
                  <Spinner />
                </div>
              )}
            </div>
          )}
        {rightPanelStatus.tab === "Template" &&
          template &&
          screen === "instructions" && (
            <div className="mb-72 flex flex-col gap-4 px-6">
              <div className="flex items-end justify-between pt-2">
                <Page.Header
                  icon={LightbulbIcon}
                  title="Template's Instructions manual"
                />
                <TemplateDropDownMenu
                  screen={screen}
                  removeTemplate={removeTemplate}
                  resetToTemplateInstructions={resetToTemplateInstructions}
                  resetToTemplateActions={resetToTemplateActions}
                  openRightPanelTab={openRightPanelTab}
                />
              </div>
              <Page.Separator />
              {template?.helpInstructions && (
                <Markdown content={template?.helpInstructions ?? ""} />
              )}
            </div>
          )}
        {rightPanelStatus.tab === "Template" &&
          template &&
          screen === "actions" && (
            <div className="mb-72 flex flex-col gap-4 px-6">
              <div className="flex items-end justify-between pt-2">
                <Page.Header
                  icon={LightbulbIcon}
                  title={"Template's Tools manual"}
                />
                <TemplateDropDownMenu
                  screen={screen}
                  removeTemplate={removeTemplate}
                  resetToTemplateInstructions={resetToTemplateInstructions}
                  resetToTemplateActions={resetToTemplateActions}
                  openRightPanelTab={openRightPanelTab}
                />
              </div>
              <Page.Separator />
              <div className="flex flex-col gap-4">
                {template && template.helpActions && (
                  <>
                    <div>
                      <Markdown
                        content={template?.helpActions ?? ""}
                        textSize="sm"
                      />
                    </div>
                    <Separator />
                  </>
                )}
                {template && template.presetActions.length > 0 && (
                  <div className="flex flex-col gap-6">
                    <Page.SectionHeader title="Add those tools" />
                    {template.presetActions.map((presetAction, index) => (
                      <div className="flex flex-col gap-2" key={index}>
                        <div>{presetAction.help}</div>
                        <TemplateAddActionButton
                          action={presetAction}
                          addAction={(presetAction) => {
                            const action = getDefaultActionConfiguration(
                              presetAction.type
                            );
                            if (!action) {
                              // Unreachable
                              return;
                            }
                            action.name = presetAction.name;
                            action.description = presetAction.description;
                            setAction({
                              type: action.noConfigurationRequired
                                ? "insert"
                                : "pending",
                              action,
                            });
                          }}
                        />
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}
        {rightPanelStatus.tab === "Performance" && agentConfigurationId && (
          <div className="ml-4 mt-4">
            <Page.SectionHeader title="Feedback" />
            <FeedbacksSection
              owner={owner}
              agentConfigurationId={agentConfigurationId}
            />
          </div>
        )}
      </div>
    </div>
  );
}

const TemplateAddActionButton = ({
  action,
  addAction,
}: {
  action: TemplateActionType;
  addAction: (action: TemplateActionType) => void;
}) => {
  const spec = ACTION_SPECIFICATIONS[action.type];
  if (!spec) {
    // Unreachable
    return null;
  }
  return (
    <div className="w-auto">
      <Button
        icon={spec.cardIcon}
        label={`Add tool “${spec.label}”`}
        size="sm"
        variant="outline"
        onClick={() => addAction(action)}
      />
    </div>
  );
};

interface TemplateDropDownMenuProps {
  openRightPanelTab: (tabName: AssistantBuilderRightPanelTab) => void;
  removeTemplate: () => Promise<void>;
  resetToTemplateActions: () => Promise<void>;
  resetToTemplateInstructions: () => Promise<void>;
  screen: BuilderScreen;
}

const TemplateDropDownMenu = ({
  openRightPanelTab,
  removeTemplate,
  resetToTemplateActions,
  resetToTemplateInstructions,
  screen,
}: TemplateDropDownMenuProps) => {
  const confirm = useContext(ConfirmContext);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button icon={MoreIcon} size="sm" variant="ghost" />
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem
          label="Close the template"
          onClick={async () => {
            const confirmed = await confirm({
              title: "Are you sure you want to close the template?",
              message:
                "Your assistant will remain as it is but will not display template's help any more.",
              validateVariant: "warning",
            });
            if (confirmed) {
              openRightPanelTab("Preview");
              await removeTemplate();
            }
          }}
          icon={XMarkIcon}
        />
        {screen === "instructions" && (
          <DropdownMenuItem
            label="Reset instructions"
            description="Set instructions back to template's default"
            onClick={async () => {
              const confirmed = await confirm({
                title: "Are you sure?",
                message:
                  "You will lose the changes you have made to the assistant's instructions and go back to the template's default settings.",
                validateVariant: "warning",
              });
              if (confirmed) {
                await resetToTemplateInstructions();
              }
            }}
            icon={MagicIcon}
          />
        )}
        {screen === "actions" && (
          <DropdownMenuItem
            label="Reset tools"
            description="Remove all tools"
            onClick={async () => {
              const confirmed = await confirm({
                title: "Are you sure?",
                message:
                  "You will lose the changes you have made to the assistant's tools.",
                validateVariant: "warning",
              });
              if (confirmed) {
                await resetToTemplateActions();
              }
            }}
            icon={MagicIcon}
          />
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
