// Trunk's initializer (index.html, data-initializer): reports the download of the
// WebAssembly module, the first and largest part of the start, to window.heronLoading.
// The module is 80 % of the bar, the bundled data the next 17 % (src/web_files.js).

const megabytes = (bytes) => (bytes / 1048576).toFixed(1);

export default function () {
  return {
    onStart: () => window.heronLoading?.("Downloading the editor"),
    onProgress: ({ current, total }) => {
      if (total > 0) {
        window.heronLoading?.(`Downloading the editor · ${megabytes(current)} of ${megabytes(total)} MB`, (0.8 * current) / total);
      }
    },
    onSuccess: () => window.heronLoading?.("Loading the bundled data", 0.8),
    onFailure: (error) => window.heronLoading?.(`Heron Brush could not be downloaded: ${error}`, null, true),
  };
}
