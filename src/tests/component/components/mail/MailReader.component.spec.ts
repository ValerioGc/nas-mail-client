import { expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import MailReader from '@/components/mail/MailReader.vue';
import { messages } from '@/i18n';
import { mail } from '@tests/fixtures/mail';

it('shows a sent mail and offers to move it to the trash', async () => {
  const wrapper = mount(MailReader, { props: { mail: mail(), folder: 'sent', busy: false } });
  expect(wrapper.get('h2').text()).toBe(mail().subject);
  expect(wrapper.get('pre').text()).toBe(mail().body);
  expect(wrapper.text()).toContain('me@example.com');
  const buttons = wrapper.findAll('button');
  expect(buttons.map((button) => button.text())).toEqual([messages.it.folders.moveToTrash]);
  await buttons[0]!.trigger('click');
  expect(wrapper.emitted('move')).toHaveLength(1);
});

it('says why a mail was not sent and lets the trash restore or delete it', async () => {
  const wrapper = mount(MailReader, {
    props: {
      mail: mail({ status: 'failed', error: '535 authentication failed' }),
      folder: 'trash',
      busy: false,
    },
  });
  expect(wrapper.get('[role="note"]').text()).toContain('535 authentication failed');
  const [restore, remove] = wrapper.findAll('button');
  expect(restore!.text()).toBe(messages.it.folders.restore);
  await remove!.trigger('click');
  expect(wrapper.emitted('delete')).toHaveLength(1);
});
