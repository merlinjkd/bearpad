<script lang="ts">
	import { Slider } from "$lib/components/ui/slider/index.js";
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Select from "$lib/components/ui/select/index.js";
	import * as ToggleGroup from "$lib/components/ui/toggle-group/index.js";

	// The Select popups are portalled. Target the sheet so they inherit its scoped tokens
	// (this modal is theme-invariant) rather than the body's theme-aware ones — otherwise
	// in light mode the dropdown would open light inside a dark modal.
	let sheetEl: HTMLDivElement | undefined = $state();

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
		{ label: 'monospace', value: 'monospace' }
	];

	const THEME_OPTIONS = [
		{ value: 'dark', label: 'Dark' },
		{ value: 'light', label: 'Light' },
		{ value: 'system', label: 'System' }
	] as const;
</script>

<!-- Styling is Tailwind utilities (migration step 4c — the <style> block is gone; its rules
     had to be deleted rather than superseded, because Svelte scopes component selectors to
     two classes and a surviving rule would outrank a single-class utility on specificity).
     The controls are shadcn-svelte components (step 5b): Slider for the font sizes, Select
     for the dropdowns, ToggleGroup for the Theme bar, Button for the actions.

     This modal is deliberately theme-invariant — its palette is hardcoded dark and does not
     follow the app theme. The shadcn tokens are therefore scoped to `.sheet` in app.css, and
     the Select popups portal into the sheet (portalProps={{ to: sheetEl }}) so they inherit
     that scope rather than the body's theme-aware tokens.

     Escape closes via the sheet's keydown; the overlay's onclick is the click-outside close. -->

<div
	class="overlay fixed inset-0 z-[9999] bg-black/50 flex items-center justify-center"
	onclick={onClose}
	role="presentation"
>
	<div
		bind:this={sheetEl}
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
			<Button
				variant="ghost"
				size="icon-xs"
				class="text-[#888]"
				onclick={onClose}
				aria-label="Close settings">✕</Button
			>
		</div>

		<div
			class="body p-5 flex flex-col gap-5 overflow-y-auto flex-1 min-h-0 [scrollbar-width:thin] [scrollbar-color:#555_#252526] [&::-webkit-scrollbar]:w-[10px] [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar-thumb]:bg-[#4a4a4a] [&::-webkit-scrollbar-thumb]:rounded-[5px] [&::-webkit-scrollbar-thumb]:border-2 [&::-webkit-scrollbar-thumb]:border-[#252526] [&::-webkit-scrollbar-thumb:hover]:bg-[#5f5f5f]"
		>
			<div class="field flex flex-col gap-2">
				<div class="radio-label text-[0.8125em] font-medium text-[#aaaaaa]">Theme</div>
				<ToggleGroup.Root
					type="single"
					value={settings.theme}
					onValueChange={(v) => {
						// a single ToggleGroup lets you deselect the active item; the theme must
						// always be one of the three, so ignore the empty value.
						if (v) onChange({ theme: v as 'dark' | 'light' | 'system' });
					}}
					variant="outline"
					spacing={0}
					aria-label="Theme"
					class="w-full overflow-hidden"
				>
					{#each THEME_OPTIONS as opt}
						<ToggleGroup.Item
							value={opt.value}
							class="flex-1 bg-[#2d2d2d] text-[#999] text-[0.8125em] data-[state=on]:bg-[#094771] data-[state=on]:text-white"
							>{opt.label}</ToggleGroup.Item
						>
					{/each}
				</ToggleGroup.Root>
			</div>

			<div class="field flex flex-col gap-2" role="group" aria-label="Editor Font Size">
				<div class="radio-label text-[0.8125em] font-medium text-[#aaaaaa]"
					>Editor Font Size: {settings.fontSize}px</div
				>
				<div class="size-controls flex items-center gap-2">
					<Button
						variant="secondary"
						size="icon"
						class="border-border"
						onclick={() => onChange({ fontSize: Math.max(10, settings.fontSize - 1) })}
						>–</Button
					>
					<Slider
						type="single"
						class="flex-1"
						min={10}
						max={32}
						step={1}
						value={settings.fontSize}
						onValueChange={(v) => onChange({ fontSize: v })}
					/>
					<Button
						variant="secondary"
						size="icon"
						class="border-border"
						onclick={() => onChange({ fontSize: Math.min(32, settings.fontSize + 1) })}
						>+</Button
					>
				</div>
			</div>

			<div class="field flex flex-col gap-2" role="group" aria-label="Menu UI Font Size">
				<div class="radio-label text-[0.8125em] font-medium text-[#aaaaaa]"
					>Menu UI Font Size: {settings.uiFontSize}px</div
				>
				<div class="size-controls flex items-center gap-2">
					<Button
						variant="secondary"
						size="icon"
						class="border-border"
						onclick={() => onChange({ uiFontSize: Math.max(12, settings.uiFontSize - 1) })}
						>–</Button
					>
					<Slider
						type="single"
						class="flex-1"
						min={12}
						max={24}
						step={1}
						value={settings.uiFontSize}
						onValueChange={(v) => onChange({ uiFontSize: v })}
					/>
					<Button
						variant="secondary"
						size="icon"
						class="border-border"
						onclick={() => onChange({ uiFontSize: Math.min(24, settings.uiFontSize + 1) })}
						>+</Button
					>
				</div>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="font-family-select"
					>Font Family</label
				>
				<Select.Root
					type="single"
					value={settings.fontFamily}
					onValueChange={(v) => onChange({ fontFamily: v })}
				>
					<Select.Trigger
						class="w-full bg-secondary dark:bg-secondary text-[0.8125em]"
						id="font-family-select"
					>
						{FONT_OPTIONS.find((o) => o.value === settings.fontFamily)?.label ??
							settings.fontFamily}
					</Select.Trigger>
					<Select.Content portalProps={{ to: sheetEl }}>
						{#each FONT_OPTIONS as opt}
							<Select.Item value={opt.value} label={opt.label}>{opt.label}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="word-wrap-select"
					>Word Wrap</label
				>
				<Select.Root
					type="single"
					value={settings.wordWrap ? 'on' : 'off'}
					onValueChange={(v) => onChange({ wordWrap: v === 'on' })}
				>
					<Select.Trigger
						class="w-full bg-secondary dark:bg-secondary text-[0.8125em]"
						id="word-wrap-select"
					>
						<Select.Value />
					</Select.Trigger>
					<Select.Content portalProps={{ to: sheetEl }}>
						<Select.Item value="on" label="On">On</Select.Item>
						<Select.Item value="off" label="Off">Off</Select.Item>
					</Select.Content>
				</Select.Root>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="spellcheck-select"
					>Spell Check</label
				>
				<Select.Root
					type="single"
					value={settings.spellcheck ? 'on' : 'off'}
					onValueChange={(v) => onChange({ spellcheck: v === 'on' })}
				>
					<Select.Trigger
						class="w-full bg-secondary dark:bg-secondary text-[0.8125em]"
						id="spellcheck-select"
					>
						<Select.Value />
					</Select.Trigger>
					<Select.Content portalProps={{ to: sheetEl }}>
						<Select.Item value="on" label="On">On</Select.Item>
						<Select.Item value="off" label="Off">Off</Select.Item>
					</Select.Content>
				</Select.Root>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="spell-lang-select"
					>Spell Check Language</label
				>
				<Select.Root
					type="single"
					value={settings.spellLang ?? 'en_US'}
					onValueChange={(v) => onChange({ spellLang: v })}
				>
					<Select.Trigger
						class="w-full bg-secondary dark:bg-secondary text-[0.8125em]"
						id="spell-lang-select"
					>
						<Select.Value />
					</Select.Trigger>
					<Select.Content portalProps={{ to: sheetEl }}>
						<Select.Item value="en_US" label="English (US)">English (US)</Select.Item>
						<Select.Item value="en_CA" label="English (Canada)">English (Canada)</Select.Item>
						<Select.Item value="en_GB" label="English (UK)">English (UK)</Select.Item>
						<Select.Item value="fr_CA" label="French (Canada)">French (Canada)</Select.Item>
						<Select.Item value="es_ES" label="Spanish">Spanish</Select.Item>
					</Select.Content>
				</Select.Root>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="cursor-blink-select"
					>Blinking Cursor</label
				>
				<Select.Root
					type="single"
					value={settings.cursorBlink ? 'on' : 'off'}
					onValueChange={(v) => onChange({ cursorBlink: v === 'on' })}
				>
					<Select.Trigger
						class="w-full bg-secondary dark:bg-secondary text-[0.8125em]"
						id="cursor-blink-select"
					>
						<Select.Value />
					</Select.Trigger>
					<Select.Content portalProps={{ to: sheetEl }}>
						<Select.Item value="off" label="Off">Off</Select.Item>
						<Select.Item value="on" label="On">On</Select.Item>
					</Select.Content>
				</Select.Root>
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
					<Button
						variant="secondary"
						size="sm"
						class="border-border text-[0.8125em]"
						onclick={() => onChange({ textColor: '' })}
						>Reset</Button
					>
				</div>
			</div>

			<div class="field flex flex-col gap-2">
				<label class="text-[0.8125em] font-medium text-[#aaaaaa]" for="default-format-select"
					>Default File Format</label
				>
				<Select.Root
					type="single"
					value={settings.defaultFormat || 'txt'}
					onValueChange={(v) => onChange({ defaultFormat: v as 'txt' | 'md' })}
				>
					<Select.Trigger
						class="w-full bg-secondary dark:bg-secondary text-[0.8125em]"
						id="default-format-select"
					>
						<Select.Value />
					</Select.Trigger>
					<Select.Content portalProps={{ to: sheetEl }}>
						<Select.Item value="txt" label="Text (.txt)">Text (.txt)</Select.Item>
						<Select.Item value="md" label="Markdown (.md)">Markdown (.md)</Select.Item>
					</Select.Content>
				</Select.Root>
			</div>
		</div>

		<div class="footer px-5 py-3 border-t border-[#3c3c3c] flex justify-end shrink-0">
			<Button class="px-5 text-[0.8125em]" onclick={onClose}>Done</Button>
		</div>
	</div>
</div>