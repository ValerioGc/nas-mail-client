<script setup lang="ts">

import { onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from '@/i18n';
import { useMailSettings } from '@/composables/useMailSettings';
import { fetchStatus, checkSmtpConnection } from '@/services/mailApi';
import { NasIcon } from '@valeriogc/nas-base-app';
import server from '@/assets/icons/server.svg?url';
import type { Security } from '@/types/mail';

/** The first configuration takes the whole page; afterwards the form is a tab of the settings. */
const props = defineProps<{ setup?: boolean }>();

const { t } = useI18n();
const { settings, saving, error, save } = useMailSettings();

const draft = reactive({
  host: '',
  port: 465,
  security: 'tls' as Security,
  username: '',
  password: '',
  from: '',
  recipients: '',
});
const msmtp = ref(true);
const testing = ref(false);
const notice = ref<string | null>(null);
const testError = ref<string | null>(null);

const securities: {
  value: Security;
  label: 'securityTls' | 'securityStarttls' | 'securityNone';
}[] = [
  { value: 'tls', label: 'securityTls' },
  { value: 'starttls', label: 'securityStarttls' },
  { value: 'none', label: 'securityNone' },
];

watch(
  settings,
  (value) => {
    if (value) {
      Object.assign(draft, {
        ...value.smtp,
        password: '',
        recipients: value.smtp.recipients.join(', '),
      });
    }
  },
  { immediate: true },
);

onMounted(async () => {
  try {
    msmtp.value = (await fetchStatus()).msmtp;
  } catch {
    // The connection check also reports a missing program.
  }
});

async function submit() {
  const current = settings.value;
  if (!current) {
    return;
  }
  notice.value = null;
  testError.value = null;
  const saved = await save({
    smtp: {
      host: draft.host.trim(),
      port: Number(draft.port),
      security: draft.security,
      username: draft.username.trim(),
      ...(draft.password ? { password: draft.password } : {}),
      from: draft.from.trim(),
      recipients: draft.recipients
        .split(',')
        .map((address) => address.trim())
        .filter(Boolean),
    },
    access: current.access,
    ui: current.ui,
    apps: current.apps,
    system: current.system,
  });
  if (saved) {
    notice.value = t.value.smtp.saved;
  }
}

async function test() {
  testing.value = true;
  notice.value = null;
  testError.value = null;
  try {
    await checkSmtpConnection();
    notice.value = t.value.smtp.testConnected;
  } catch (reason) {
    testError.value = t.value.smtp.testFailed.replace(
      '{error}',
      reason instanceof Error ? reason.message : String(reason),
    );
  } finally {
    testing.value = false;
  }
}

</script>

<template>
  <section class="smtp" :class="{ smtp_setup: props.setup }" data-testid="smtp">
    <header v-if="props.setup" class="smtp_welcome">
      <span class="smtp_symbol"><NasIcon :svg="server" /></span>
      <h1>{{ t.setup.title }}</h1>
      <p>{{ t.setup.intro }}</p>
    </header>
    <p v-if="!msmtp" class="smtp_warning" role="alert">{{ t.smtp.msmtpMissing }}</p>

    <form class="smtp_form" @submit.prevent="submit">
      <label class="field field_wide">
        <span>{{ t.smtp.host }}</span>
        <input
          v-model="draft.host"
          name="host"
          type="text"
          autocomplete="off"
          placeholder="smtp.example.com"
        />
      </label>
      <label class="field">
        <span>{{ t.smtp.port }}</span>
        <input v-model.number="draft.port" name="port" type="number" min="1" max="65535" required />
      </label>
      <label class="field">
        <span>{{ t.smtp.security }}</span>
        <select v-model="draft.security" name="security">
          <option v-for="item in securities" :key="item.value" :value="item.value">
            {{ t.smtp[item.label] }}
          </option>
        </select>
      </label>
      <label class="field">
        <span>{{ t.smtp.username }}</span>
        <input v-model="draft.username" name="username" type="text" autocomplete="off" />
      </label>
      <label class="field">
        <span>{{ t.smtp.password }}</span>
        <input
          v-model="draft.password"
          name="password"
          type="password"
          autocomplete="new-password"
          :placeholder="settings?.passwordSet ? t.smtp.passwordKept : ''"
        />
      </label>
      <label class="field field_wide">
        <span>{{ t.smtp.from }}</span>
        <input v-model="draft.from" name="from" type="email" placeholder="nas@example.com" />
      </label>
      <label class="field field_wide">
        <span>{{ t.smtp.recipients }}</span>
        <input
          v-model="draft.recipients"
          name="recipients"
          type="text"
          aria-describedby="smtp-recipients-hint"
        />
        <small id="smtp-recipients-hint">{{ t.smtp.recipientsHint }}</small>
      </label>

      <p v-if="error" class="smtp_error field_wide" role="alert">{{ error }}</p>
      <p v-if="testError" class="smtp_error field_wide" role="alert">{{ testError }}</p>
      <output v-if="notice" class="smtp_notice field_wide">{{ notice }}</output>

      <footer class="field_wide">
        <button
          type="button"
          class="app_button"
          :disabled="testing || saving || !settings?.configured"
          data-testid="smtp_test"
          @click="test"
        >
          {{ t.smtp.test }}
        </button>
        <button
          type="submit"
          class="app_button app_button--primary"
          :disabled="saving || !settings"
          data-testid="smtp_save"
        >
          {{ props.setup ? t.setup.save : t.smtp.save }}
        </button>
      </footer>
    </form>
  </section>
</template>

<style scoped lang="scss">

.smtp_setup {
  width: min(100%, 760px);
  margin: 0 auto;
}

.smtp_welcome {
  display: grid;
  justify-items: center;
  gap: 8px;
  margin-bottom: 20px;
  text-align: center;

  h1 {
    margin: 0;
    font-size: 24px;
  }

  p {
    max-width: 34rem;
    margin: 0;
    color: var(--nas-muted);
  }
}

.smtp_symbol {
  --app-icon-size: 28px;
  display: grid;
  place-items: center;
  width: 60px;
  height: 60px;
  border-radius: 50%;
  background: var(--accent-soft);
  color: var(--accent, #3c8fbe);
}

.smtp_warning,
.smtp_error {
  margin: 0 0 16px;
  color: var(--nas-error);
}

.smtp_notice {
  margin: 0;
  color: var(--nas-success, var(--nas-muted));
}

.smtp_form {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  padding: 20px;
  border: 1px solid var(--nas-border);
  border-radius: 10px;
  background: var(--nas-surface);

  footer {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }
}

// In the settings the dialog is the card already.
.smtp:not(.smtp_setup) .smtp_form {
  padding: 0;
  border: 0;
  background: transparent;
}

.field {
  display: grid;
  gap: 6px;
  min-width: 0;

  span {
    font-weight: 600;
  }

  input,
  select {
    min-height: 40px;
    padding: 0 12px;
    border: 1px solid var(--nas-border);
    border-radius: 8px;
    background: var(--nas-background);
    color: var(--nas-foreground);
    font: inherit;
  }

  small {
    color: var(--nas-muted);
  }
}

.field_wide {
  grid-column: 1 / -1;
}

@media (max-width: 700px) {
  .smtp_form {
    grid-template-columns: minmax(0, 1fr);
  }
}

</style>
