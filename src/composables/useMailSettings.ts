import { ref } from 'vue';
import { locale } from '@/i18n';
import { fetchSettings, saveSettings } from '@/services/mailApi';
import type { MailSettings, SettingsView } from '@/types/mail';

// Shared by every section: the SMTP form and the three sections of mails edit the same settings.
const settings = ref<SettingsView | null>(null);
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);

async function load() {
  loading.value = true;
  try {
    settings.value = await fetchSettings();
    error.value = null;
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
  } finally {
    loading.value = false;
  }
}

/** Saves the settings in the language of the page, which the mails then speak. */
async function save(next: Omit<MailSettings, 'language'>): Promise<boolean> {
  saving.value = true;
  try {
    settings.value = await saveSettings({ ...next, language: locale.value });
    error.value = null;
    return true;
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
    return false;
  } finally {
    saving.value = false;
  }
}

export function useMailSettings() {
  return { settings, loading, saving, error, load, save };
}
