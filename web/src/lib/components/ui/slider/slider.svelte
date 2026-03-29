<script lang="ts">
	import { Slider as SliderPrimitive } from "bits-ui";
	import { cn, type WithoutChildrenOrChild } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		value = $bindable(),
		orientation = "horizontal",
		class: className,
		...restProps
	}: WithoutChildrenOrChild<SliderPrimitive.RootProps> = $props();
</script>

<!--
Discriminated Unions + Destructing (required for bindable) do not
get along, so we shut typescript up by casting `value` to `never`.
-->
<SliderPrimitive.Root
	bind:ref
	bind:value={value as never}
	data-slot="slider"
	{orientation}
	class={cn(
		"group/slider relative flex w-full touch-none items-center select-none data-[disabled]:opacity-50 data-[orientation=vertical]:h-full data-[orientation=vertical]:min-h-44 data-[orientation=vertical]:w-auto data-[orientation=vertical]:flex-col motion-reduce:transition-none",
		className
	)}
	{...restProps}
>
	{#snippet children({ thumbs })}
		<span
			data-orientation={orientation}
			data-slot="slider-track"
			class={cn(
				"bg-white/10 relative grow overflow-hidden rounded-full transition-transform duration-200 ease-out group-hover/slider:data-[orientation=horizontal]:scale-y-[1.18] group-focus-within/slider:data-[orientation=horizontal]:scale-y-[1.18] group-hover/slider:data-[orientation=vertical]:scale-x-[1.18] group-focus-within/slider:data-[orientation=vertical]:scale-x-[1.18] motion-reduce:transition-none data-[orientation=horizontal]:h-1.5 data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-1.5"
			)}
		>
			<SliderPrimitive.Range
				data-slot="slider-range"
				class={cn(
					"bg-primary absolute transition-[transform,filter,width,height,left,right,top,bottom] duration-150 ease-out group-hover/slider:saturate-110 group-focus-within/slider:saturate-110 motion-reduce:transition-none data-[orientation=horizontal]:h-full data-[orientation=horizontal]:origin-left data-[orientation=vertical]:w-full data-[orientation=vertical]:origin-bottom"
				)}
			/>
		</span>
		{#each thumbs as thumb (thumb)}
			<SliderPrimitive.Thumb
				data-slot="slider-thumb"
				index={thumb}
				class="opacity-100 transition-[left,right,top,bottom,transform] duration-150 ease-out hover:scale-110 focus-visible:scale-110 active:scale-95 motion-reduce:transition-none"
			/>
		{/each}
	{/snippet}
</SliderPrimitive.Root>

<style>
	:global([data-slot='slider-thumb']) {
		will-change: transform, left, right, top, bottom;
	}

	:global([data-slot='slider-range']) {
		will-change: width, height, left, right, top, bottom, transform;
	}
</style>
