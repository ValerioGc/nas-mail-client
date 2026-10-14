<script setup lang="ts">

import { ref, watch } from 'vue';
import { useI18n } from '@/i18n';
import CommonModal from '@/components/common/CommonModal.vue';
import MailEvents from '@/components/settings/MailEvents.vue';
import SmtpSettings from '@/components/settings/SmtpSettings.vue';
import type { SettingsTab } from '@/types/mail';

const props = withDefaults(defineProps<{ initialTab?: SettingsTab }>(), {
  initialTab: 'account',
});
const open = defineModel<boolean>({ required: true });

const { t } = useI18n();
const tabs: readonly SettingsTab[] = ['account', 'notifications'];
const tab = ref<SettingsTab>(props.initialTab);

watch(open, (shown) => {
  if (shown) {
    tab.value = props.initialTab;
  }
});

/** The arrows, Home and End move between the tabs, as in the other settings of the NAS. */
function keyboard(event: KeyboardEvent) {
  const index = tabs.indexOf(tab.value);
  const next = {
    ArrowRight: (index + 1) % tabs.length,
    ArrowLeft: (index - 1 + tabs.length) % tabs.length,
    Home: 0,
    End: tabs.length - 1,
  }[event.key];
  if (next === undefined) {
    return;
  }
  event.preventDefault();
  tab.value = tabs[next] ?? 'account';
  (event.currentTarget as HTMLElement)
    .querySelector<HTMLButtonElement>(`[data-tab="${tab.value}"]`)
    ?.focus();
}

</script>

<template>
  <CommonModal v-model="open" :title="t.settings.title">
    <div class="settings_tabs" role="tablist" :aria-label="t.settings.title" @keydown="keyboard">
      <button
        v-for="item in tabs"
        :id="`settings_tab_${item}`"
        :key="item"
        type="button"
        class="settings_tab"
        role="tab"
        :aria-selected="tab === item"
        :tabindex="tab === item ? 0 : -1"
        :aria-controls="`settings_panel_${item}`"
        :data-tab="item"
        :data-testid="`settings_tab_${item}`"
        @click="tab = item"
      >
        {{ t.settings.tabs[item] }}
      </button>
    </div>

    <div :id="`settings_panel_${tab}`" role="tabpanel" :aria-labelledby="`settings_tab_${tab}`">
      <SmtpSettings v-if="tab === 'account'" />
      <MailEvents v-else />
    </div>
  </CommonModal>
</template>

<style scoped lang="scss">

.settings_tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 20px;
  border-bottom: 1px solid var(--nas-border);
}

.settings_tab {
  margin-bottom: -1px;
  padding: 8px 14px;
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--nas-muted);
  font-weight: 600;

  &:hover {
    color: var(--nas-foreground);
  }

  &[aria-selected='true'] {
    border-bottom-color: var(--accent, #3c8fbe);
    color: var(--nas-foreground);
  }
}

</style>
