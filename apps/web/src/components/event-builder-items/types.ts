import type { OptionLink } from "@/types";

export type Option = {
  id: string;
  label: string;
  imageUrl?: string | null;
  imageAssetUrl?: string | null;
  links?: OptionLink[];
};
export type Item = {
  id: string;
  kind: "single_choice" | "numeric" | "multiple_choice";
  title: string;
  lockAt: string;
  revealAt: string;
  correctOptionId: string | null;
  options: Option[];
  decimalPlaces?: number;
  unitLabel?: string | null;
  minValue?: string | null;
  maxValue?: string | null;
  resultValue?: string | null;
  minSelections?: number;
  maxSelections?: number | null;
};

export type MediaDraft = { imageUrl: string; links: OptionLink[] };
export type Action = (path: string, body?: unknown) => Promise<boolean>;
export type SetState<T> = React.Dispatch<React.SetStateAction<T>>;

export type EventBuilderItemsProps = {
  draft: { event: { id: string }; items: Item[] };
  editable: boolean;
  mediaEditable: boolean;
  busy: boolean;
  editingItemId: string | null;
  itemTitleDraft: string;
  setItemTitleDraft: SetState<string>;
  itemLockDraft: string;
  setItemLockDraft: SetState<string>;
  editingOptionId: string | null;
  optionLabelDraft: string;
  setOptionLabelDraft: SetState<string>;
  openMediaOptionId: string | null;
  setOpenMediaOptionId: SetState<string | null>;
  mediaDrafts: Record<string, MediaDraft>;
  setMediaDrafts: SetState<Record<string, MediaDraft>>;
  openAddOptionItemId: string | null;
  setOpenAddOptionItemId: SetState<string | null>;
  labels: Record<string, string>;
  setLabels: SetState<Record<string, string>>;
  results: Record<string, string>;
  setResults: SetState<Record<string, string>>;
  multipleResults: Record<string, string[]>;
  setMultipleResults: SetState<Record<string, string[]>>;
  action: Action;
  load: (id: string) => Promise<void>;
  addOption: (item: Item) => Promise<void>;
  startItemEdit: (item: Item) => void;
  cancelItemEdit: () => void;
  saveItemEdit: (item: Item) => Promise<void>;
  startOptionEdit: (option: Option) => void;
  cancelOptionEdit: () => void;
  saveOptionLabel: (item: Item, option: Option) => Promise<void>;
  saveOptionMedia: (item: Item, option: Option) => Promise<void>;
};
