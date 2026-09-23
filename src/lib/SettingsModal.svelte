<script lang="ts">
	interface SettingsData {
		theme: 'dark' | 'light' | 'system';
		fontSize: number;
		uiFontSize: number;
		fontFamily: string;
		wordWrap: boolean;
		spellcheck: boolean;
		spellLang?: string;
		cursorBlink?: boolean;
		textColor?: string;
		defaultFormat?: 'txt' | 'md';
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
		{ label: 'Consolas', value: "\"Consolas\", 'SF Mono', 'Fira Code', monospace" },
		{ label: 'SF Mono', value: "'SF Mono', 'Fira Code', monospace" },
		{ label: 'Fira Code', value: "'Fira Code', 'Fira Code VF', monospace" },
		{ label: 'Cascadia Code', value: "'Cascadia Code', 'Cascadia Code PL', monospace" },
		{ label: 'JetBrains Mono', value: "'JetBrains Mono', monospace" },
		{ label: 'Monaco', value: "'Monaco', monospace" },
		{ label: 'monospace', value: 'monospace' },
	];
</script>

<!-- All styling is Tailwind utilities (migration step 4c). The <style> block is
     gone; its rules had to be deleted rather than superseded: Svelte scopes
     component selectors to two classes, so a surviving rule would outrank a
     single-class utility on specificity and silently defeat it. Colors are
     intentionally the same hardcoded values as before — this modal was already
     theme-invariant (no vars), and the migration must not change appearance.

     Escape closes via the sheet's keydown; the overlay's onclick is the
     click-outside close. Both preserved verbatim. -->

<div
	class="overlay fixed inset-0 z-[9999] bg-black/50 flex items-center justify-center"
	onclick={onClose}
	role="presentation"
>
	<div
		class="sheet bg-[#252526] border border-[#3c3c3c] rounded-[10px] min-w-[420px] max-w-[500px] max-h-[90vh] flex flex-col shadow-[0_16px_48px_rgba(0,0,0,0.6)] font-[-apple-system,BlinkMacSystemFont,'Segoe_UI',system-ui,sans-serif] text-[#cccccc] text-ui"
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => {
			if (e.key === 'Escape') onClose();
		}}
		role="dialog"
		aria-label="Settings"
		tabindex="-1"
	>
		<div class="header flex items-center justify-between px-5 py-4 border-b border-[#3c3c3c]">
			<h2 class="m-0 text-[0.9375em] font-semibold text-white">Settings</h2>
			<button
				class="close-btn bg-transparent border-0 text-[#888] text-[1em] cursor-pointer py-0.5 px-1.5 rounded hover:bg-[#3c3c3c] hover:text-white"
				onclick={onClose}
				>✕</button
			>
		</div>

		<div
			class="body p-5 flex flex-col gap-5 overflow-y-auto flex-1 min-h-0 [scrollbar-width:thin] [scrollbar-color:#555_#252526] [&::-webkit-scrollbar]:w-[10px] [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar-thumb]:bg-[#4a4a4a] [&::-webkit-scrollbar-thumb]:rounded-[5px] [&::-webkit-scrollbar-thumb]:border-2 [&::-webkit-scrollbar-thumb]:border-[#252526] [&::-webkit-scrollbar-thumb:hover]:bg-[#5f5f5f]"
		>
			<div class="field flex flex-col gap-2" role="group" aria-label="Theme">
				<div class="radio-label text-[0.8125em] font-medium text-[#aaaaaa]">Theme</div>
				<div class="radio-group flex rounded-md overflow-hidden border border-[#3c3c3c]">
					<button
						class="radio-btn flex-1 py-1.5 px-3 border-0 bg-[#2d2d2d] text-[#999] cursor-pointer text-[0.8125em] transition-[background,color] duration-150 not-last:border-r not-last:border-[#3c3c3c] {settings.theme === 'dark' ? 'bg-[#094771] text-white' : ''}"
						onclick={() => onChange({ theme: 'dark' })}
						>Dark</button
					>
					<button
						class="radio-btn flex-1 py-1.5 px-3 border-0 bg-[#2d2d2d] text-[#999] cursor-pointer text-[0.8125em] transition-[background,color] duration-150 not-last:border-r not-last:border-[#3c3c3c] {settings.theme === 'light' ? 'bg-[#094771] text-white' : ''}"
						onclick={() => onChange({ theme: 'light' })}
						>Light</button
					>
					<button
						class="radio-btn flex-1 py-1.5 px-3 border-0 bg-[#2d2d2d] text-[#999] cursor-pointer text-[0.8125em] transition-[background,color] duration-150 not-last:border-r not-last:border-[#3c3c3c] {settings.theme === 'system' ? 'bg-[#094771] text-white' : ''}"
						onclick={() => onChange({ theme: 'system' })}
						>System</button
					>
				</div>
			</div>

			<div class="field flex flex-col gap-2" role="group" aria-label="Editor Font Size">
				<div class="radio-label text-[0.8125em] font-medium text-[#aaaaaa]"
					>Editor Font Size: {settings.fontSize}px</div
				>
				<div class="size-controls flex items-center gap-2">
					<button
						class="size-btn w-8 h-8 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[1em] cursor-pointer flex items-center justify-center hover:bg-[#3c3c3c]"
						onclick={() => onChange({ fontSize: Math.max(10, settings.fontSize - 1) })}
						>–</button
					>
					<input
						class="flex-1 accent-[#094771] h-1"
						type="range"
						min="10"
						max="32"
						step="1"
						value={settings.fontSize}
						oninput={(e) => onChange({ fontSize: parseInt((e.target as HTMLInputElement).value) })}
					/>
					<button
						class="size-btn w-8 h-8 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[1em] cursor-pointer flex items-center justify-center hover:bg-[#3c3c3c]"
						onclick={() => onChange({ fontSize: Math.min(32, settings.fontSize + 1) })}
						>+</button
					>
				</div>
			</div>

			<div class="field flex flex-col gap-2" role="group" aria-label="Menu UI Font Size">
				<div class="radio-label text-[0.8125em] font-medium text-[#aaaaaa]"
					>Menu UI Font Size: {settings.uiFontSize}px</div
				>
				<div class="size-controls flex items-center gap-2">
					<button
						class="size-btn w-8 h-8 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[1em] cursor-pointer flex items-center justify-center hover:bg-[#3c3c3c]"
						onclick={() => onChange({ uiFontSize: Math.max(12, settings.uiFontSize - 1) })}
						>–</button
					>
					<input
						class="flex-1 accent-[#094771] h-1"
						type="range"
						min="12"
						max="24"
						step="1"
						value={settings.uiFontSize}
						oninput={(e) => onChange({ uiFontSize: parseInt((e.target as HTMLInputElement).value) })}
					/>
					<button
						class="size-btn w-8 h-8 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[1em] cursor-pointer flex items-center justify-center hover:bg-[#3c3c3c]"
						onclick={() => onChange({ uiFontSize: Math.min(24, settings.uiFontSize + 1) })}
						>+</button
					>
				</div>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="font-family-select"
					>Font Family</label
				>
				<select
					class="py-1.5 px-2.5 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[0.8125em]"
					id="font-family-select"
					value={settings.fontFamily}
					onchange={(e) => onChange({ fontFamily: (e.target as HTMLSelectElement).value })}
				>
					{#each FONT_OPTIONS as opt}
						<option value={opt.value}>{opt.label}</option>
					{/each}
				</select>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="word-wrap-select"
					>Word Wrap</label
				>
				<select
					class="py-1.5 px-2.5 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[0.8125em]"
					id="word-wrap-select"
					value={settings.wordWrap ? 'on' : 'off'}
					onchange={(e) =>
						onChange({ wordWrap: (e.target as HTMLSelectElement).value === 'on' })}
				>
					<option value="on">On</option>
					<option value="off">Off</option>
				</select>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="spellcheck-select"
					>Spell Check</label
				>
				<select
					class="py-1.5 px-2.5 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[0.8125em]"
					id="spellcheck-select"
					value={settings.spellcheck ? 'on' : 'off'}
					onchange={(e) =>
						onChange({ spellcheck: (e.target as HTMLSelectElement).value === 'on' })}
				>
					<option value="on">On</option>
					<option value="off">Off</option>
				</select>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="spell-lang-select"
					>Spell Check Language</label
				>
				<select
					class="py-1.5 px-2.5 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[0.8125em]"
					id="spell-lang-select"
					value={settings.spellLang ?? 'en_US'}
					onchange={(e) => onChange({ spellLang: (e.target as HTMLSelectElement).value })}
				>
					<option value="en_US">English (US)</option>
					<option value="en_CA">English (Canada)</option>
					<option value="en_GB">English (UK)</option>
					<option value="fr_CA">French (Canada)</option>
					<option value="es_ES">Spanish</option>
				</select>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="cursor-blink-select"
					>Blinking Cursor</label
				>
				<select
					class="py-1.5 px-2.5 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[0.8125em]"
					id="cursor-blink-select"
					value={settings.cursorBlink ? 'on' : 'off'}
					onchange={(e) => onChange({ cursorBlink: (e.target as HTMLSelectElement).value === 'on' })}
				>
					<option value="off">Off</option>
					<option value="on">On</option>
				</select>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="text-color-input"
					>Text Color (blank = theme default)</label
				>
				<div class="color-row flex items-center gap-2">
					<input
						class="w-[44px] h-[28px] p-0.5 border border-[#3c3c3c] rounded-md bg-[#2d2d2d] cursor-pointer"
						id="text-color-input"
						type="color"
						value={settings.textColor || '#d4d4d4'}
						onchange={(e) => onChange({ textColor: (e.target as HTMLInputElement).value })}
					/>
					<button
						class="color-reset py-1 px-3 border border-[#3c3c3c] rounded-md bg-[#2d2d2d] text-[#cccccc] text-[0.8125em] cursor-pointer hover:bg-[#3c3c3c]"
						onclick={() => onChange({ textColor: '' })}
						>Reset</button
					>
				</div>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="default-format-select"
					>Default File Format</label
				>
				<select
					class="py-1.5 px-2.5 rounded-md border border-[#3c3c3c] bg-[#2d2d2d] text-[#cccccc] text-[0.8125em]"
					id="default-format-select"
					value={settings.defaultFormat || 'txt'}
					onchange={(e) => onChange({ defaultFormat: (e.target as HTMLSelectElement).value as 'txt' | 'md' })}
				>
					<option value="txt">Text (.txt)</option>
					<option value="md">Markdown (.md)</option>
				</select>
			</div>
		</div>

		<div class="footer px-5 py-3 border-t border-[#3c3c3c] flex justify-end shrink-0">
			<button
				class="action-btn py-1.5 px-5 rounded-md border-0 bg-[#094771] text-white text-[0.8125em] cursor-pointer hover:bg-[#1a5a8a]"
				onclick={onClose}
				>Done</button
			>
		</div>
	</div>
</div>