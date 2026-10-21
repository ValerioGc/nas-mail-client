import type { Mail, MailSettings, SettingsView } from '@/types/mail';

export function mailSettings(): MailSettings {
  return {
    language: 'it',
    smtp: {
      host: 'smtp.example.com',
      port: 465,
      security: 'tls',
      username: 'nas@example.com',
      from: 'nas@example.com',
      recipients: ['me@example.com'],
    },
    access: { refused: false },
    ui: { available: false, updated: false, failed: false },
    apps: { available: false },
    system: { available: false, level: 'security' },
  };
}

export function settingsView(overrides: Partial<SettingsView> = {}): SettingsView {
  return { ...mailSettings(), passwordSet: true, configured: true, ...overrides };
}

export function mail(overrides: Partial<Mail> = {}): Mail {
  return {
    id: 1,
    category: 'access',
    recipients: 'me@example.com',
    subject: 'Accesso al NAS rifiutato',
    body: 'Un tentativo di accesso da 10.0.0.9 è stato rifiutato.',
    sentAt: Date.UTC(2026, 9, 8, 10, 0),
    status: 'sent',
    error: null,
    trashedAt: null,
    ...overrides,
  };
}

export function answer(body: unknown, status = 200) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), { status }),
  );
}
