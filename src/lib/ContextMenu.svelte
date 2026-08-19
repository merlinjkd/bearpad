<script lang="ts">
	import { onMount } from 'svelte';

	let { contextMenu, onhide }: { contextMenu: { x: number; y: number; items: any[] }; onhide: () => void } = $props();

	let menuRef: HTMLDivElement;
	// Anchor below the cursor by default; flip above if it would overflow the window.
	let up = $state(false);
	// Flip left if it would overflow the right edge.
	let left = $state(false);

	onMount(() => {
		const r = menuRef.getBoundingClientRect();
		const mh = r.height;
		const mw = r.width;
		const winW = window.innerWidth;
		const winH = window.innerHeight;
		up = contextMenu.y + mh > winH - 4;
		left = contextMenu.x + mw > winW - 4;
	});
</script>

<div
	class="custom-context-menu"
	class:open-up={up}
	class:open-left={left}
	style="left: {contextMenu.x}px; top: {contextMenu.y}px"
	bind:this={menuRef}
	role="menu"
>
	{#each contextMenu.items as item}
		{#if item.separator}
			<hr />
		{:else}
			<button
				class="menu-item"
				disabled={item.disabled}
				onclick={() => {
					if (!item.disabled && item.onClick) {
						item.onClick();
					}
					onhide();
				}}
			>
				<span class="label">{item.label}</span>
				{#if item.shortcut}
					<span class="shortcut">{item.shortcut}</span>
				{/if}
			</button>
		{/if}
	{/each}
</div>

<style>
	.custom-context-menu {
		position: fixed;
		z-index: 10000;
		background: #252526;
		border: 1px solid #3c3c3c;
		border-radius: 6px;
		padding: 4px 0;
		min-width: 220px;
		box-shadow: 0 8px 24px rgba(0,0,0,0.5);
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
		font-size: var(--ui-font-size, 16px);
	}
	/* flip up: translate by its own height so the menu sits above the cursor */
	.custom-context-menu.open-up {
		transform: translateY(-100%);
	}
	/* flip left: shift back by its own width */
	.custom-context-menu.open-left {
		transform: translateX(-100%);
	}
	/* both flips */
	.custom-context-menu.open-up.open-left {
		transform: translate(-100%, -100%);
	}
	hr {
		margin: 4px 8px;
		border: none;
		border-top: 1px solid #3c3c3c;
	}
	.menu-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 4px 16px;
		border: none;
		background: transparent;
		color: #cccccc;
		cursor: pointer;
		text-align: left;
		box-sizing: border-box;
	}
	.menu-item:hover:not(:disabled) {
		background: #094771;
		color: #ffffff;
	}
	.menu-item:disabled {
		color: #5a5a5a;
		cursor: default;
	}
	.label {
		flex: 1;
	}
	.shortcut {
		margin-left: 32px;
		color: #6e6e6e;
		font-size: 0.875em;
	}
	.menu-item:hover:not(:disabled) .shortcut {
		color: #a0a0a0;
	}
</style>
