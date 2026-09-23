<script lang="ts">
	let { contextMenu, onhide }: { contextMenu: { x: number; y: number; items: any[] }; onhide: () => void } = $props();

	// Split the app window at 50%. Cursor in the top half -> menu drops down;
	// cursor in the bottom half -> menu pops up. Fixed/relative to the window
	// (clientY is viewport-relative and the menu is position:fixed, so 50% of
	// innerHeight is the natural midpoint). Keeping the flip class-based lets
	// the menu anchor at the cursor and shift by its own size.
	// ponytail: symmetric 50% split; pick per-side weights later if optical
	// balance matters, but 50% is the standard and reads predictably.
	const winH = window.innerHeight;
	const up = contextMenu.y > winH / 2;
	const left = contextMenu.x > window.innerWidth - 260;
</script>

<!-- All styling is Tailwind utilities (migration step 4d). The <style> block is
     gone; its rules had to be deleted rather than superseded (Svelte-scoped rules
     outrank single-class utilities on specificity).

     The custom-context-menu class MUST stay: App.svelte's document click handler
     does .closest('.custom-context-menu') to decide whether a click was inside the
     menu. The edge-flip is now two conditional translate utilities instead of the
     open-up/open-left classes — Tailwind composes translate-x and translate-y, so
     both flips together still produce translate(-100%, -100%) exactly like the old
     .open-up.open-left rule. -->

<div
	class="custom-context-menu fixed z-[10000] bg-[#252526] border border-[#3c3c3c] rounded-md py-1 min-w-[220px] shadow-[0_8px_24px_rgba(0,0,0,0.5)] font-[-apple-system,BlinkMacSystemFont,'Segoe_UI',system-ui,sans-serif] text-ui {up ? 'translate-y-[-100%]' : ''} {left ? 'translate-x-[-100%]' : ''}"
	style="left: {contextMenu.x}px; top: {contextMenu.y}px"
	role="menu"
>
	{#each contextMenu.items as item}
		{#if item.separator}
			<hr class="my-1 mx-2 border-t border-[#3c3c3c]" />
		{:else}
			<button
				class="menu-item group flex items-center justify-between w-full py-1 px-4 border-0 bg-transparent text-[#cccccc] cursor-pointer text-left box-border enabled:hover:bg-[#094771] enabled:hover:text-white disabled:text-[#5a5a5a] disabled:cursor-default"
				disabled={item.disabled}
				onclick={() => {
					if (!item.disabled && item.onClick) {
						item.onClick();
					}
					onhide();
				}}
			>
				<span class="label flex-1">{item.label}</span>
				{#if item.shortcut}
					<span class="shortcut ml-8 text-[#6e6e6e] text-[0.875em] group-[&:enabled]:hover:text-[#a0a0a0]"
						>{item.shortcut}</span
					>
				{/if}
			</button>
		{/if}
	{/each}
</div>