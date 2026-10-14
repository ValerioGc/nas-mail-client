import { beforeEach, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import MailEvents from '@/components/settings/MailEvents.vue';
import { useMailSettings } from '@/composables/useMailSettings';
import { messages } from '@/i18n';
import { saveSettings } from '@/services/mailApi';
import { settingsView } from '@tests/fixtures/mail';

vi.mock('@/services/mailApi', () => ({ fetchSettings: vi.fn(), saveSettings: vi.fn() }));

const t = messages.it.events;

beforeEach(() => {
  vi.clearAllMocks();
  useMailSettings().settings.value = settingsView();
  useMailSettings().error.value = null;
  vi.mocked(saveSettings).mockImplementation(async (saved) => ({
    ...settingsView(),
    ...saved,
  }));
});

it('groups every automatic mail in one tab, each with its switch', () => {
  const wrapper = mount(MailEvents);
  expect(wrapper.findAll('h3').map((heading) => heading.text())).toEqual([
    messages.it.sections.access,
    messages.it.sections.ui,
    messages.it.sections.apps,
    messages.it.sections.system,
  ]);
  expect(wrapper.findAll('[role="switch"]')).toHaveLength(6);
  expect(wrapper.get('[data-testid="events_ui"]').text()).toContain(t.failed);
  expect(wrapper.get('[data-testid="events_apps"]').text()).toContain(t.appsAvailable);
});

it('saves the switches with the account untouched', async () => {
  const wrapper = mount(MailEvents);
  await wrapper.get('[data-testid="choice_refused"]').setValue(true);
  await wrapper.get('[data-testid="choice_failed"]').setValue(true);
  await wrapper.get('[data-testid="choice_apps_available"]').setValue(true);
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  const saved = vi.mocked(saveSettings).mock.lastCall?.[0];
  expect(saved?.access).toEqual({ refused: true });
  expect(saved?.ui).toEqual({ available: false, updated: false, failed: true });
  expect(saved?.apps).toEqual({ available: true });
  expect(saved?.smtp).toEqual(settingsView().smtp);
  expect(wrapper.get('output').text()).toBe(t.saved);
});

it('chooses the minimum level of system updates once their mails are on', async () => {
  const wrapper = mount(MailEvents);
  expect(wrapper.get('fieldset').attributes('disabled')).toBeDefined();
  await wrapper.get('[data-testid="choice_system_available"]').setValue(true);
  expect(wrapper.get('fieldset').attributes('disabled')).toBeUndefined();
  await wrapper.get('input[value="stable"]').setValue(true);
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(vi.mocked(saveSettings).mock.lastCall?.[0].system).toEqual({
    available: true,
    level: 'stable',
  });
});

it('shows a refused save, and nothing before the settings are read', async () => {
  vi.mocked(saveSettings).mockRejectedValueOnce(new Error('invalid'));
  const wrapper = mount(MailEvents);
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toBe('invalid');

  useMailSettings().settings.value = null;
  await flushPromises();
  expect(wrapper.find('form').exists()).toBe(false);
});
