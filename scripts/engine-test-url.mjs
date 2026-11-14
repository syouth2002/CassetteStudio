// Obtain the actual OS-assigned port; never rely on a fixed test port.
export async function engineUrl(server) {
  let logs = '';
  server.stdout.on('data', b => logs += b);
  server.stderr.on('data', b => logs += b);
  for (let i = 0; i < 200; i++) {
    const url = logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
    if (url) return url;
    if (server.exitCode !== null) throw Error(logs || 'Engine exited before startup');
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw Error('Engine startup timeout: ' + logs);
}
