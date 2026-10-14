import type { Folder, Mail, MailSettings, MailStatus, SettingsView } from '@/types/mail';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isMail(value: unknown): value is Mail {
  return (
    isRecord(value) &&
    typeof value.id === 'number' &&
    ['access', 'ui', 'system', 'test'].includes(value.category as string) &&
    typeof value.recipients === 'string' &&
    typeof value.subject === 'string' &&
    typeof value.body === 'string' &&
    typeof value.sentAt === 'number' &&
    (value.status === 'sent' || value.status === 'failed') &&
    (value.error === null || typeof value.error === 'string') &&
    (value.trashedAt === null || typeof value.trashedAt === 'number')
  );
}

function isSettings(value: unknown): value is SettingsView {
  return (
    isRecord(value) &&
    isRecord(value.smtp) &&
    typeof value.smtp.host === 'string' &&
    Array.isArray(value.smtp.recipients) &&
    isRecord(value.access) &&
    isRecord(value.ui) &&
    isRecord(value.apps) &&
    isRecord(value.system) &&
    typeof value.passwordSet === 'boolean' &&
    typeof value.configured === 'boolean'
  );
}

function isStatus(value: unknown): value is MailStatus {
  return (
    isRecord(value) &&
    typeof value.msmtp === 'boolean' &&
    typeof value.configured === 'boolean' &&
    typeof value.sender === 'boolean'
  );
}

/** Calls the backend and returns its JSON, or throws the message of its error. */
async function call(path: string, init?: RequestInit): Promise<unknown> {
  const response = await fetch(path, init);
  const text = await response.text();
  let payload: unknown = null;
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      throw new Error(`Invalid backend answer (HTTP ${response.status}).`);
    }
  }
  if (!response.ok) {
    throw new Error(
      isRecord(payload) && typeof payload.error === 'string'
        ? payload.error
        : `Backend request failed (HTTP ${response.status}).`,
    );
  }
  return payload;
}

function checked<T>(payload: unknown, valid: (value: unknown) => value is T): T {
  if (!valid(payload)) {
    throw new Error('Invalid backend answer.');
  }
  return payload;
}

export async function fetchStatus(): Promise<MailStatus> {
  return checked(await call('api/status'), isStatus);
}

export async function fetchSettings(): Promise<SettingsView> {
  return checked(await call('api/settings'), isSettings);
}

export async function saveSettings(settings: MailSettings): Promise<SettingsView> {
  const payload = await call('api/settings', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(settings),
  });
  return checked(payload, isSettings);
}

export async function sendTestMail(): Promise<Mail> {
  return checked(await call('api/test', { method: 'POST' }), isMail);
}

export async function fetchMails(folder: Folder): Promise<Mail[]> {
  const payload = await call(`api/mails?folder=${folder}`);
  return checked(payload, (value): value is Mail[] => Array.isArray(value) && value.every(isMail));
}

export async function moveMail(id: number, to: Folder): Promise<void> {
  await call(`api/mails/${id}/${to === 'trash' ? 'trash' : 'restore'}`, { method: 'POST' });
}

export async function deleteMail(id: number): Promise<void> {
  await call(`api/mails/${id}`, { method: 'DELETE' });
}

export async function emptyTrash(): Promise<void> {
  await call('api/trash', { method: 'DELETE' });
}
