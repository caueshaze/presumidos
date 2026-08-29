import { EventBuilderItemCard } from "./event-builder-items/EventBuilderItemCard";

export type { Item, Option } from "./event-builder-items/types";
import type { EventBuilderItemsProps as Props } from "./event-builder-items/types";

export function EventBuilderItems(props: Props) {
  return (
    <div className="mt-4 flex flex-col gap-4">
      {props.draft.items.map((item, index) => (
        <EventBuilderItemCard key={item.id} item={item} index={index} state={props} />
      ))}
    </div>
  );
}
