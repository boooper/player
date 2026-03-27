<script lang="ts">
	import { getEmblaContext } from "./context.js";
	import { cn } from "$lib/utils.js";

	let {
		href,
		onclick,
		label = "See all",
		class: className,
	}: {
		href?: string;
		onclick?: () => void;
		label?: string;
		class?: string;
	} = $props();

	const emblaCtx = getEmblaContext("<Carousel.SeeAll/>");

	$effect(() => {
		emblaCtx.hasSeeAll = true;
		return () => { emblaCtx.hasSeeAll = false; };
	});

	const baseClasses = "absolute top-0 end-0 -translate-y-full flex h-8 items-center gap-1 px-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:text-foreground";
</script>

{#if href}
	<a {href} class={cn(baseClasses, className)}>
		{label}
	</a>
{:else}
	<button type="button" {onclick} class={cn(baseClasses, className)}>
		{label}
	</button>
{/if}
