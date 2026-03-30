<script lang="ts">
	import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
	import { Tooltip, TooltipTrigger, TooltipContent } from "$lib/components/ui/tooltip";

	let { ref = $bindable(null), tooltip, ...restProps }: DropdownMenuPrimitive.TriggerProps & { tooltip?: string } = $props();
</script>

{#if tooltip}
	<Tooltip>
		<TooltipTrigger>
			{#snippet child({ props })}
				<DropdownMenuPrimitive.Trigger bind:ref data-slot="dropdown-menu-trigger" {...restProps} {...props} />
			{/snippet}
		</TooltipTrigger>
		<TooltipContent>{tooltip}</TooltipContent>
	</Tooltip>
{:else}
	<DropdownMenuPrimitive.Trigger bind:ref data-slot="dropdown-menu-trigger" {...restProps} />
{/if}
