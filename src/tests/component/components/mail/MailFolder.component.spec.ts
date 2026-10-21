import { beforeEach, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import MailFolder from '@/components/mail/MailFolder.vue';
import { messages } from '@/i18n';
import { deleteMail, emptyTrash, fetchMails, moveMail } from '@/services/mailApi';
import { stubDialog } from '@tests/component/helpers/dialog';
import { mail } from '@tests/fixtures/mail';

vi.mock('@/services/mailApi', () => ({
  fetchMails: vi.fn(),
  moveMail: vi.fn(),
  deleteMail: vi.fn(),
  emptyTrash: vi.fn(),
}));

beforeEach(() => {
  stubDialog();
  vi.clearAllMocks();
});

const t = messages.it.folders;

it('lists the sent mails, opens one and moves it to the trash', async () => {
  vi.mocked(fetchMails)
    .mockResolvedValueOnce([
      mail(),
      mail({ id: 2, subject: 'Second', status: 'failed', error: 'x' }),
    ])
    .mockResolvedValueOnce([mail({ id: 2, subject: 'Second' })]);
  const wrapper = mount(MailFolder, { props: { folder: 'sent' } });
  await flushPromises();
  expect(fetchMails).toHaveBeenCalledWith('sent');
  expect(wrapper.text()).toContain(t.selectMail);
  const items = wrapper.findAll('.mail_list button');
  expect(items).toHaveLength(2);
  expect(items[1]!.text()).toContain(t.failed);

  await items[0]!.trigger('click');
  expect(wrapper.get('[data-testid="mail_reader"] h2').text()).toBe(mail().subject);
  await wrapper.get('[data-testid="mail_reader"] button').trigger('click');
  await flushPromises();
  expect(moveMail).toHaveBeenCalledWith(1, 'trash');
  expect(wrapper.findAll('.mail_list button')).toHaveLength(1);
  expect(wrapper.find('[data-testid="mail_reader"]').exists()).toBe(false);
});

it('restores, deletes after confirmation and empties the trash', async () => {
  vi.mocked(fetchMails).mockResolvedValue([mail({ trashedAt: 5 })]);
  const wrapper = mount(MailFolder, { props: { folder: 'trash' } });
  await flushPromises();
  const open = () => wrapper.get('.mail_list button').trigger('click');

  await open();
  await wrapper.findAll('[data-testid="mail_reader"] button')[0]!.trigger('click');
  await flushPromises();
  expect(moveMail).toHaveBeenCalledWith(1, 'sent');

  await open();
  await wrapper.findAll('[data-testid="mail_reader"] button')[1]!.trigger('click');
  expect(wrapper.get('dialog h2').text()).toBe(t.deleteTitle);
  await wrapper.get('dialog .app_button--primary').trigger('click');
  await flushPromises();
  expect(deleteMail).toHaveBeenCalledWith(1);

  await wrapper.get('[data-testid="empty_trash"]').trigger('click');
  expect(wrapper.get('dialog h2').text()).toBe(t.emptyTrashTitle);
  await wrapper.get('dialog button').trigger('click');
  expect(wrapper.find('dialog').exists()).toBe(false);
  await wrapper.get('[data-testid="empty_trash"]').trigger('click');
  await wrapper.get('dialog .app_button--primary').trigger('click');
  await flushPromises();
  expect(emptyTrash).toHaveBeenCalledOnce();
});

it('shows an empty folder, and an error with a retry', async () => {
  vi.mocked(fetchMails).mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce([]);
  const wrapper = mount(MailFolder, { props: { folder: 'sent' } });
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toContain('offline');
  await wrapper.get('[role="alert"] button').trigger('click');
  await flushPromises();
  expect(wrapper.get('.empty_state h2').text()).toBe(t.sentEmpty);

  vi.mocked(fetchMails).mockResolvedValueOnce([]);
  await wrapper.setProps({ folder: 'trash' });
  await flushPromises();
  expect(wrapper.get('.empty_state h2').text()).toBe(t.trashEmpty);
  expect(wrapper.find('[data-testid="empty_trash"]').exists()).toBe(false);
});

it('keeps the mails when an action fails and says why', async () => {
  vi.mocked(fetchMails).mockResolvedValue([mail()]);
  vi.mocked(moveMail).mockRejectedValueOnce(new Error('locked'));
  const wrapper = mount(MailFolder, { props: { folder: 'sent' } });
  await flushPromises();
  await wrapper.get('.mail_list button').trigger('click');
  await wrapper.get('[data-testid="mail_reader"] button').trigger('click');
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toContain('locked');
});
