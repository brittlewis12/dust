import { defineSignal } from "@temporalio/workflow";

export const newFoldersSelectionSignal = defineSignal<[void]>(
  "new_folders_selection_signal"
);
export const newWebhookSignal = defineSignal<[void]>("new_webhook_signal");
