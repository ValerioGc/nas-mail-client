<script setup lang="ts">

import { NasIcon, NasModal } from '@valeriogc/nas-base-app';
import { useI18n } from '@/i18n';
import close from '@/assets/icons/close.svg?url';

defineProps<{ title: string }>();

const open = defineModel<boolean>({ required: true });

const { t } = useI18n();

</script>

<template>
  <NasModal v-if="open" :label="title" data-testid="modal" @close="open = false">
    <section class="modal">
      <header class="modal_heading">
        <h2>{{ title }}</h2>
        <button
          type="button"
          class="modal_close"
          :aria-label="t.folders.close"
          :title="t.folders.close"
          data-testid="modal_close"
          @click="open = false"
        >
          <NasIcon :svg="close" />
        </button>
      </header>

      <div class="modal_body">
        <slot></slot>
      </div>
    </section>
  </NasModal>
</template>

<style scoped lang="scss">

.modal {
  display: flex;
  flex-direction: column;
  width: min($nas_settings_modal_width, 100%);
  height: $nas_settings_modal_height;
  max-height: calc(100vh - 40px);
  overflow: hidden;
  border: 1px solid var(--nas-border);
  border-radius: 12px;
  background: var(--nas-surface);
  color: var(--nas-foreground);
  box-shadow: 0 16px 48px #0003;
}

.modal_heading {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 24px;
  border-bottom: 1px solid var(--nas-border);

  h2 {
    margin: 0;
    font-size: 17px;
  }
}

.modal_close {
  --app-icon-size: 20px;
  display: inline-grid;
  place-items: center;
  min-width: 32px;
  min-height: 32px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--nas-muted);

  &:hover {
    background: var(--nas-hover);
    color: var(--nas-foreground);
  }
}

.modal_body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 20px 24px;
}

</style>
