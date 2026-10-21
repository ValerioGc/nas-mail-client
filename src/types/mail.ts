export type Security = 'tls' | 'starttls' | 'none';
export type Level = 'security' | 'stable' | 'all';
export type Folder = 'sent' | 'trash';
export type EventSection = 'access' | 'ui' | 'apps' | 'system';
export type SettingsTab = 'account' | 'notifications';
export type MailCategory = EventSection | 'test';

export interface Smtp {
  host: string;
  port: number;
  security: Security;
  username: string;
  password?: string;
  from: string;
  recipients: string[];
}

export interface MailSettings {
  language: 'it' | 'en';
  smtp: Smtp;
  access: { refused: boolean };
  ui: { available: boolean; updated: boolean; failed: boolean };
  apps: { available: boolean };
  system: { available: boolean; level: Level };
}

export interface SettingsView extends MailSettings {
  passwordSet: boolean;
  configured: boolean;
}

export interface Mail {
  id: number;
  category: MailCategory;
  recipients: string;
  subject: string;
  body: string;
  sentAt: number;
  status: 'sent' | 'failed';
  error: string | null;
  trashedAt: number | null;
}

export interface MailStatus {
  msmtp: boolean;
  configured: boolean;
  sender: boolean;
}
