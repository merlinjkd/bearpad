<script lang="ts">
	interface SettingsData {
		theme: 'dark' | 'light' | 'system' | 'github-dark';
		fontSize: number;
		uiFontSize: number;
		fontFamily: string;
		wordWrap: boolean;
		spellcheck: boolean;
	}

	let {
		settings,
		onClose,
		onChange,
	}: {
		settings: SettingsData;
		onClose: () => void;
		onChange: (patch: Partial<SettingsData>) => void;
	} = $props();

	const FONT_OPTIONS = [
		{ label: 'SF Mono', value: "'SF Mono', 'Fira Code', monospace" },
		{ label: 'Fira Code', value: "'Fira Code', 'Fira Code VF', monospace" },
		{ label: 'Cascadia Code', value: "'Cascadia Code', 'Cascadia Code PL', monospace" },
		{ label: 'JetBrains Mono', value: "'JetBrains Mono', monospace" },
		{ label: 'Monaco', value: "'Monaco', monospace" },
		{ label: 'monospace', value: 'monospace' },
	];
</script>

<div class="overlay" onclick={onClose} role="presentation">
	<div class="sheet" onclick={(e) => e.stopPropagation()} onkeydown={(e) => { if (e.key === 'Escape') onClose(); }} role="dialog" aria-label="Settings" tabindex="-1">
		<div class="header">
			<h2>Settings</h2>
			<button class="close-btn" onclick={onClose}>✕</button>
		</div>

		<div class="body">
			<div class="field" role="group" aria-label="Theme">
				<div class="radio-label">Theme</div>
				<div class="radio-group">
					<button
						class="radio-btn"
						class:active={settings.theme === 'dark'}
						onclick={() => onChange({ theme: 'dark' })}
					>Dark</button>
					<button
						class="radio-btn"
						class:active={settings.theme === 'light'}
						onclick={() => onChange({ theme: 'light' })}
					>Light</button>
					<button
						class="radio-btn"
						class:active={settings.theme === 'system'}
						onclick={() => onChange({ theme: 'system' })}
					>System</button>
					<button
						class="radio-btn"
						class:active={settings.theme === 'github-dark'}
						onclick={() => onChange({ theme: 'github-dark' })}
					>GitHub Dark</button>
				</div>
			</div>

			<div class="field" role="group" aria-label="Font Size">
				<div class="radio-label">Font Size: {settings.fontSize}px</div>
				<div class="size-controls">
					<button class="size-btn" onclick={() => onChange({ fontSize: Math.max(10, settings.fontSize - 1) })}>–</button>
					<input
						type="range"
						min="10"
						max="32"
						step="1"
						value={settings.fontSize}
						oninput={(e) => onChange({ fontSize: parseInt((e.target as HTMLInputElement).value) })}
					/>
					<button class="size-btn" onclick={() => onChange({ fontSize: Math.min(32, settings.fontSize + 1) })}>+</button>
				</div>
			</div>

			<div class="field" role="group" aria-label="UI Font Size">
				<div class="radio-label">UI Font Size: {settings.uiFontSize}px</div>
				<div class="size-controls">
					<button class="size-btn" onclick={() => onChange({ uiFontSize: Math.max(12, settings.uiFontSize - 1) })}>–</button>
					<input
						type="range"
						min="12"
						max="24"
						step="1"
						value={settings.uiFontSize}
						oninput={(e) => onChange({ uiFontSize: parseInt((e.target as HTMLInputElement).value) })}
					/>
					<button class="size-btn" onclick={() => onChange({ uiFontSize: Math.min(24, settings.uiFontSize + 1) })}>+</button>
				</div>
			</div>

			<div class="field">
				<label for="font-family-select">Font Family</label>
				<select
					id="font-family-select"
					value={settings.fontFamily}
					onchange={(e) => onChange({ fontFamily: (e.target as HTMLSelectElement).value })}
				>
					{#each FONT_OPTIONS as opt}
						<option value={opt.value}>{opt.label}</option>
					{/each}
				</select>
			</div>

			<div class="field">
				<label for="word-wrap-select">Word Wrap</label>
				<select
					id="word-wrap-select"
					value={settings.wordWrap ? 'on' : 'off'}
					onchange={(e) =>
						onChange({ wordWrap: (e.target as HTMLSelectElement).value === 'on' })}
				>
					<option value="on">On</option>
					<option value="off">Off</option>
				</select>
			</div>

			<div class="field">
				<label for="spellcheck-select">Spell Check (English)</label>
				<select
					id="spellcheck-select"
					value={settings.spellcheck ? 'on' : 'off'}
					onchange={(e) =>
						onChange({ spellcheck: (e.target as HTMLSelectElement).value === 'on' })}
				>
					<option value="on">On</option>
					<option value="off">Off</option>
				</select>
			</div>
		</div>

		<div class="footer">
			<button class="action-btn" onclick={onClose}>Done</button>
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 9999;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.sheet {
		background: #252526;
		border: 1px solid #3c3c3c;
		border-radius: 10px;
		min-width: 420px;
		max-width: 500px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.6);
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
		color: #cccccc;
		font-size: var(--ui-font-size, 16px);
	}
	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid #3c3c3c;
	}
	.header h2 {
		margin: 0;
		font-size: 0.9375em;
		font-weight: 600;
		color: #ffffff;
	}
	.close-btn {
		background: none;
		border: none;
		color: #888;
		font-size: 1em;
		cursor: pointer;
		padding: 2px 6px;
		border-radius: 4px;
	}
	.close-btn:hover {
		background: #3c3c3c;
		color: #fff;
	}
	.body {
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.field label,
	.radio-label {
		font-size: 0.8125em;
		font-weight: 500;
		color: #aaaaaa;
	}
	.radio-group {
		display: flex;
		gap: 0;
		border-radius: 6px;
		overflow: hidden;
		border: 1px solid #3c3c3c;
	}
	.radio-btn {
		flex: 1;
		padding: 6px 12px;
		border: none;
		background: #2d2d2d;
		color: #999;
		cursor: pointer;
		font-size: 0.8125em;
		transition: background 0.15s, color 0.15s;
	}
	.radio-btn:not(:last-child) {
		border-right: 1px solid #3c3c3c;
	}
	.radio-btn.active {
		background: #094771;
		color: #ffffff;
	}
	.size-controls {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.size-btn {
		width: 32px;
		height: 32px;
		border-radius: 6px;
		border: 1px solid #3c3c3c;
		background: #2d2d2d;
		color: #cccccc;
		font-size: 1em;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.size-btn:hover {
		background: #3c3c3c;
	}
	input[type="range"] {
		flex: 1;
		accent-color: #094771;
		height: 4px;
	}
	select {
		padding: 6px 10px;
		border-radius: 6px;
		border: 1px solid #3c3c3c;
		background: #2d2d2d;
		color: #cccccc;
		font-size: 0.8125em;
	}
	.footer {
		padding: 12px 20px;
		border-top: 1px solid #3c3c3c;
		display: flex;
		justify-content: flex-end;
	}
	.action-btn {
		padding: 6px 20px;
		border-radius: 6px;
		border: none;
		background: #094771;
		color: #ffffff;
		font-size: 0.8125em;
		cursor: pointer;
	}
	.action-btn:hover {
		background: #1a5a8a;
	}
</style>