<script setup lang="ts">

import { computed, onMounted, ref } from 'vue';
import { NasAppLayout, NasIcon, type NasCategory } from '@valeriogc/nas-base-app';
import { useNasLauncherContext } from '@/composables/useNasLauncherContext';
import { useMailSettings } from '@/composables/useMailSettings';
import type { Folder, SettingsTab } from '@/types/mail';

import MailFolder from '@/components/mail/MailFolder.vue';
import SettingsModal from '@/components/settings/SettingsModal.vue';
import SmtpSettings from '@/components/settings/SmtpSettings.vue';

import sent from '@/assets/icons/sent.svg?url';
import settingsIcon from '@/assets/icons/settings.svg?url';
import trash from '@/assets/icons/trash.svg?url';

const { t } = useNasLauncherContext();
const { settings, loading, error, load } = useMailSettings();

const folder = ref<Folder>('sent');
const settingsOpen = ref(false);
const folders = computed<NasCategory<Folder>[]>(() => [
  { value: 'sent', label: t.value.sections.sent, icon: sent, testid: 'section_sent' },
  { value: 'trash', label: t.value.sections.trash, icon: trash, testid: 'section_trash' },
]);

const asked = new URL(window.location.href).searchParams.get('section');

const TABS: Record<string, SettingsTab> = {
  smtp: 'account',
  access: 'notifications',
  ui: 'notifications',
  apps: 'notifications',
  system: 'notifications',
};

const settingsTab = ref<SettingsTab>(TABS[asked ?? ''] ?? 'account');

onMounted(async () => {
  await load();
  if (asked && TABS[asked] && settings.value?.configured) {
    settingsOpen.value = true;
  }
});

</script>

<template>
  <div v-if="error && !settings" class="app_surface app_surface--alone">
    <main class="app_content load_error" role="alert">
      <span>{{ error }}</span>
      <button type="button" class="app_button app_button--small" :disabled="loading" @click="load">
        {{ t.folders.retry }}
      </button>
    </main>
  </div>

  <div v-else-if="!settings" class="app_surface app_surface--alone">
    <main class="app_content">
      <output class="loading_state">{{ t.loading }}</output>
    </main>
  </div>

  <div v-else-if="!settings.configured" class="app_surface app_surface--alone" data-testid="setup">
    <main class="app_content">
      <SmtpSettings setup />
    </main>
  </div>

  <template v-else>
    <NasAppLayout v-model="folder" :categories="folders" :navigation-label="t.sectionsLabel">
      <template #sidebar-header>
        <h2 class="nav_heading">{{ t.groups.mailbox }}</h2>
      </template>
      <template #sidebar-footer>
        <div class="sidebar_footer">
          <button
            type="button"
            class="category_button"
            data-testid="settings_open"
            @click="settingsOpen = true"
          >
            <NasIcon class="category_icon" :svg="settingsIcon" />
            {{ t.settings.title }}
          </button>
        </div>
      </template>
      <MailFolder :folder="folder" />
    </NasAppLayout>

    <SettingsModal v-model="settingsOpen" :initial-tab="settingsTab" />
  </template>
</template>

<style scoped lang="scss">

.app_surface--alone {
  grid-template-columns: minmax(0, 1fr);
}

.nav_heading {
  margin: 0 0 0 12px;
  color: var(--nas-muted);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
}

.sidebar_footer {
  margin-top: auto;
  padding-top: 12px;
  border-top: 1px solid var(--nas-border);
}

.load_error {
  display: flex;
  align-content: start;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  color: var(--nas-error);
}

.loading_state {
  display: block;
  margin: 1em 0;
  color: var(--nas-muted);
}

@media (max-width: 760px) {
  .nav_heading {
    display: none;
  }

  .sidebar_footer {
    margin: 0;
    padding: 0;
    border: 0;
  }
}

</style>
