<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import ChevronRightIcon from "@lucide/svelte/icons/chevron-right";
	import { cn, type WithoutChild } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		inset,
		children,
		...restProps
	}: WithoutChild<ContextMenuPrimitive.SubTriggerProps> & {
		inset?: boolean;
	} = $props();
</script>

<ContextMenuPrimitive.SubTrigger
	bind:ref
	data-slot="context-menu-sub-trigger"
	data-inset={inset}
	class={cn(
		"liquid-glass-item data-highlighted:text-foreground data-[state=open]:text-foreground [&_svg:not([class*='text-'])]:text-muted-foreground/70 flex cursor-default items-center gap-2.5 rounded-xl px-2.5 py-2 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-40 data-[inset]:ps-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
		className
	)}
	{...restProps}
>
	{@render children?.()}
	<ChevronRightIcon class="ms-auto" />
</ContextMenuPrimitive.SubTrigger>
