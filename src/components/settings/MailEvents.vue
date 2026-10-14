<script setup lang="ts">

import { computed, ref, toRaw, watch } from 'vue';
import { useI18n } from '@/i18n';
import { useMailSettings } from '@/composables/useMailSettings';
import { NasSwitch } from '@valeriogc/nas-base-app';
import type { EventSection, Level, MailSettings } from '@/types/mail';

const { t } = useI18n();
const { settings, saving, error, save } = useMailSettings();

type Choices = Pick<MailSettings, EventSection>;
const draft = ref<Choices | null>(null);
const saved = ref(false);

const levels = [
  { value: 'security', label: 'levelSecurity', hint: 'levelSecurityHint' },
  { value: 'stable', label: 'levelStable', hint: 'levelStableHint' },
  { value: 'all', label: 'levelAll', hint: 'levelAllHint' },
] as const satisfies readonly { value: Level; label: string; hint: string }[];

interface Choice {
  id: string;
  label: string;
  checked: boolean;
  set: (checked: boolean) => void;
}

// The groups of switches, each bound to its field of the draft.
const groups = computed<{ section: EventSection; choices: Choice[] }[]>(() => {
  const current = draft.value;
  if (!current) {
    return [];
  }
  const { access, ui, apps, system } = current;
  return [
    {
      section: 'access',
      choices: [
        {
          id: 'refused',
          label: t.value.events.refused,
          checked: access.refused,
          set: (checked) => (access.refused = checked),
        },
      ],
    },
    {
      section: 'ui',
      choices: (['available', 'updated', 'failed'] as const).map((key) => ({
        id: key,
        label: t.value.events[key],
        checked: ui[key],
        set: (checked: boolean) => (ui[key] = checked),
      })),
    },
    {
      section: 'apps',
      choices: [
        {
          id: 'apps_available',
          label: t.value.events.appsAvailable,
          checked: apps.available,
          set: (checked) => (apps.available = checked),
        },
      ],
    },
    {
      section: 'system',
      choices: [
        {
          id: 'system_available',
          label: t.value.events.systemAvailable,
          checked: system.available,
          set: (checked) => (system.available = checked),
        },
      ],
    },
  ];
});

watch(
  settings,
  (value) => {
    saved.value = false;
    draft.value = value
      ? structuredClone({
          access: toRaw(value.access),
          ui: toRaw(value.ui),
          apps: toRaw(value.apps),
          system: toRaw(value.system),
        })
      : null;
  },
  { immediate: true },
);

function toggle(choice: Choice, checked: boolean) {
  choice.set(checked);
  saved.value = false;
}

async function submit() {
  const current = settings.value;
  if (!current || !draft.value) {
    return;
  }
  // The page never holds the password: without one, the backend keeps the stored one.
  saved.value = await save({ smtp: current.smtp, ...draft.value });
}

</script>

<template>
  <form v-if="draft" class="events" data-testid="notifications" @submit.prevent="submit">
    <p class="events_description">{{ t.events.intro }}</p>

    <section
      v-for="group in groups"
      :key="group.section"
      class="events_group"
      :data-testid="`events_${group.section}`"
    >
      <header>
        <h3>{{ t.sections[group.section] }}</h3>
        <p>{{ t.events.description[group.section] }}</p>
      </header>

      <NasSwitch
        v-for="choice in group.choices"
        :key="choice.id"
        :model-value="choice.checked"
        :label="choice.label"
        :testid="`choice_${choice.id}`"
        @update:model-value="toggle(choice, $event)"
      />

      <fieldset
        v-if="group.section === 'system'"
        class="levels"
        :disabled="!draft.system.available"
      >
        <legend>{{ t.events.level }}</legend>
        <label v-for="level in levels" :key="level.value" class="level">
          <input
            v-model="draft.system.level"
            type="radio"
            name="system-level"
            :value="level.value"
            @change="saved = false"
          />
          <span>
            {{ t.events[level.label] }}
            <small>{{ t.events[level.hint] }}</small>
          </span>
        </label>
      </fieldset>
    </section>

    <p v-if="error" class="events_error" role="alert">{{ error }}</p>
    <output v-else-if="saved" class="events_saved">{{ t.events.saved }}</output>

    <footer>
      <button
        type="submit"
        class="app_button app_button--primary"
        :disabled="saving"
        data-testid="events_save"
      >
        {{ t.events.save }}
      </button>
    </footer>
  </form>
</template>

<style scoped lang="scss">

.events {
  display: grid;
  gap: 12px;

  footer {
    display: flex;
    justify-content: flex-end;
  }
}

.events_description {
  margin: 0;
  color: var(--nas-muted);
  line-height: 1.5;
}

.events_group {
  display: grid;
  justify-items: start;
  gap: 12px;
  padding: 16px 18px;
  border: 1px solid var(--nas-border);
  border-radius: 10px;
  background: var(--nas-sunken);

  header {
    display: grid;
    gap: 2px;
  }

  h3 {
    margin: 0;
    font-size: 15px;
  }

  header p {
    margin: 0;
    color: var(--nas-muted);
    font-size: 13px;
  }
}

.levels {
  display: grid;
  gap: 10px;
  justify-self: stretch;
  margin: 0;
  padding: 12px 16px;
  border: 1px solid var(--nas-border);
  border-radius: 8px;

  legend {
    padding: 0 4px;
    font-weight: 600;
  }

  &:disabled {
    opacity: 0.55;
  }
}

.level {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  cursor: pointer;

  input {
    margin-top: 3px;
    accent-color: var(--accent, #3c8fbe);
  }

  small {
    display: block;
    color: var(--nas-muted);
  }
}

.events_error {
  margin: 0;
  color: var(--nas-error);
}

.events_saved {
  margin: 0;
  color: var(--nas-muted);
}

</style>
