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
