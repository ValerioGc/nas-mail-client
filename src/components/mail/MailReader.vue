<script setup lang="ts">

import { useI18n } from '@/i18n';
import type { Folder, Mail } from '@/types/mail';
import { NasIcon } from '@valeriogc/nas-base-app';
import restore from '@/assets/icons/restore.svg?url';
import trash from '@/assets/icons/trash.svg?url';

defineProps<{
  mail: Mail;
  folder: Folder;
  busy: boolean;
}>();

const emit = defineEmits<{
  move: [];
  delete: [];
}>();

const { t, locale } = useI18n();

</script>

<template>
  <article class="reader" data-testid="mail_reader">
    <header class="reader_header">
      <h2>{{ mail.subject }}</h2>
      <div class="reader_actions">
        <button
          type="button"
          class="app_button app_button--small"
          :disabled="busy"
          @click="emit('move')"
        >
          <NasIcon :svg="folder === 'sent' ? trash : restore" />
          {{ folder === 'sent' ? t.folders.moveToTrash : t.folders.restore }}
        </button>
        <button
          v-if="folder === 'trash'"
          type="button"
          class="app_button app_button--small"
          :disabled="busy"
          @click="emit('delete')"
        >
          {{ t.folders.deleteForever }}
        </button>
      </div>
    </header>
    <dl class="reader_meta">
      <dt>{{ t.folders.to }}</dt>
      <dd>{{ mail.recipients }}</dd>
      <dt>{{ t.folders.date }}</dt>
      <dd>{{ new Date(mail.sentAt).toLocaleString(locale) }}</dd>
    </dl>
    <p v-if="mail.status === 'failed'" class="reader_failed" role="note">
      {{ t.folders.failed }}: {{ mail.error }}
    </p>
    <pre class="reader_body">{{ mail.body }}</pre>
  </article>
</template>

<style scoped lang="scss">

.reader {
  min-width: 0;
  padding: 20px;
  border: 1px solid var(--nas-border);
  border-radius: 10px;
  background: var(--nas-surface);
}

.reader_header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;

  h2 {
    margin: 0;
    font-size: 19px;
    overflow-wrap: anywhere;
  }
}

.reader_actions {
  --app-icon-size: 15px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.reader_meta {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 4px 12px;
  margin: 16px 0;
  color: var(--nas-muted);
  font-size: 0.875rem;

  dt {
    font-weight: 600;
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
}

.reader_failed {
  margin: 0 0 16px;
  color: var(--nas-error);
}

.reader_body {
  margin: 0;
  padding-top: 16px;
  border-top: 1px solid var(--nas-border);
  font: inherit;
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

</style>
