<script setup lang="ts">

import { computed, ref, watch } from 'vue';
import { useI18n } from '@/i18n';
import { deleteMail, emptyTrash, fetchMails, moveMail } from '@/services/mailApi';
import { NasConfirmDialog, NasEmptyState, NasIcon } from '@valeriogc/nas-base-app';
import type { Folder, Mail } from '@/types/mail';

import MailReader from '@/components/mail/MailReader.vue';

import sent from '@/assets/icons/sent.svg?url';
import trash from '@/assets/icons/trash.svg?url';

const props = defineProps<{ folder: Folder }>();

const { t, locale } = useI18n();

const mails = ref<Mail[]>([]);
const selectedId = ref<number | null>(null);
const loading = ref(false);
const busy = ref(false);
const error = ref<string | null>(null);
const confirming = ref<'delete' | 'empty' | null>(null);

const selected = computed(() => mails.value.find((mail) => mail.id === selectedId.value) ?? null);

async function load() {
  loading.value = true;
  try {
    mails.value = await fetchMails(props.folder);
    error.value = null;
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.folder,
  () => {
    selectedId.value = null;
    mails.value = [];
    void load();
  },
  { immediate: true },
);

async function act(action: () => Promise<void>) {
  busy.value = true;
  try {
    await action();
    selectedId.value = null;
    await load();
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
  } finally {
    busy.value = false;
  }
}

function moveSelected() {
  const id = selectedId.value;
  if (id !== null) {
    void act(() => moveMail(id, props.folder === 'sent' ? 'trash' : 'sent'));
  }
}

function confirmed() {
  const id = selectedId.value;
  const what = confirming.value;
  confirming.value = null;
  
  if (what === 'empty') {
    void act(emptyTrash);
  } else if (id !== null) {
    void act(() => deleteMail(id));
  }
}

</script>

<template>
  <section class="folder" :aria-busy="loading || busy">
    <header class="folder_header">
      <h1>{{ t.sections[folder] }}</h1>
      <button
        v-if="folder === 'trash' && mails.length"
        type="button"
        class="app_button"
        :disabled="busy"
        data-testid="empty_trash"
        @click="confirming = 'empty'"
      >
        {{ t.folders.emptyTrash }}
      </button>
    </header>

    <div v-if="error" class="folder_error" role="alert">
      <span>{{ error }}</span>
      <button type="button" class="app_button app_button--small" :disabled="loading" @click="load">
        {{ t.folders.retry }}
      </button>
    </div>
    <output v-else-if="loading && !mails.length" class="folder_status">
      {{ t.folders.loading }}
    </output>
    <NasEmptyState
      v-else-if="!mails.length"
      class="empty_state"
      :title="folder === 'sent' ? t.folders.sentEmpty : t.folders.trashEmpty"
      :description="t.folders.emptyHint"
    >
      <template #icon>
        <NasIcon class="empty_icon" :svg="folder === 'sent' ? sent : trash" />
      </template>
    </NasEmptyState>
    <div v-else class="folder_body">
      <ul class="mail_list" :aria-label="t.sections[folder]">
        <li v-for="mail in mails" :key="mail.id">
          <button
            type="button"
            :class="['mail_item', { active: mail.id === selectedId }]"
            :aria-pressed="mail.id === selectedId"
            @click="selectedId = mail.id"
          >
            <span class="mail_subject">{{ mail.subject }}</span>
            <span class="mail_meta">
              {{ t.categories[mail.category] }} · {{ new Date(mail.sentAt).toLocaleString(locale) }}
            </span>
            <span v-if="mail.status === 'failed'" class="mail_failed">{{ t.folders.failed }}</span>
          </button>
        </li>
      </ul>
      <MailReader
        v-if="selected"
        :mail="selected"
        :folder="folder"
        :busy="busy"
        @move="moveSelected"
        @delete="confirming = 'delete'"
      />
      <p v-else class="reader_placeholder">{{ t.folders.selectMail }}</p>
    </div>

    <NasConfirmDialog
      v-if="confirming"
      :title="confirming === 'empty' ? t.folders.emptyTrashTitle : t.folders.deleteTitle"
      :message="confirming === 'empty' ? t.folders.emptyTrashMessage : t.folders.deleteMessage"
      :cancel-label="t.folders.cancel"
      :confirm-label="confirming === 'empty' ? t.folders.emptyTrash : t.folders.deleteForever"
      @cancel="confirming = null"
      @confirm="confirmed"
    />
  </section>
</template>

<style scoped lang="scss">

.folder_header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 24px;

  h1 {
    margin: 0;
    font-size: 24px;
  }
}

.folder_error {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  color: var(--nas-error);
}

.folder_status {
  display: block;
  margin: 1em 0;
  color: var(--nas-muted);
}

.reader_placeholder {
  color: var(--nas-muted);
}

.empty_state {
  gap: 0;
  padding: 64px 16px;

  :deep(h2) {
    margin: 16px 0 6px;
    font-size: 18px;
  }
}

.empty_icon {
  --app-icon-size: 48px;
  opacity: 0.45;
}

.folder_body {
  display: grid;
  grid-template-columns: minmax(220px, 340px) minmax(0, 1fr);
  align-items: start;
  gap: 20px;
}

.mail_list {
  display: grid;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.mail_item {
  display: grid;
  gap: 4px;
  width: 100%;
  padding: 12px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: var(--nas-foreground);
  text-align: left;

  &:hover,
  &.active {
    background: var(--nas-hover);
  }

  &.active {
    border-color: var(--accent, #3c8fbe);
  }
}

.mail_subject {
  overflow: hidden;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mail_meta {
  color: var(--nas-muted);
  font-size: 0.8125rem;
}

.mail_failed {
  color: var(--nas-error);
  font-size: 0.8125rem;
}

@media (max-width: 900px) {
  .folder_body {
    grid-template-columns: minmax(0, 1fr);
  }
}

</style>
