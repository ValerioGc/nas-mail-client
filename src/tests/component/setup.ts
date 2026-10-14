// jsdom does not implement the native dialog methods. Browser tests cover focus and modality.
Object.defineProperties(HTMLDialogElement.prototype, {
  showModal: {
    configurable: true,
    writable: true,
    value(this: HTMLDialogElement) {
      this.open = true;
    },
  },
  close: {
    configurable: true,
    writable: true,
    value(this: HTMLDialogElement) {
      this.open = false;
    },
  },
});
