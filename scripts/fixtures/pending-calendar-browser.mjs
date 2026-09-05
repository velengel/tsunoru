// Deliberately never resolves launch, exercising the pre-browser shutdown window.
export const chromium = {
  launch() {
    // The verifier has started its owned server and selected a disposable cwd.
    console.log('pending_browser_launch=true');
    return new Promise(() => {});
  },
};
