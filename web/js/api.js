"use strict";
// HTTP requests and serialized collection persistence.
async function api(path, body, method = "POST") {
  const response = await fetch(
    path,
    body === undefined
      ? {}
      : {
          method,
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        },
  );
  let data;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      csText("サーバーから応答を取得できません ({0})", response.status),
    );
  }
  if (!response.ok)
    throw new Error(
      data.error || csText("処理に失敗しました ({0})", response.status),
    );
  return data;
}
function persist(c) {
  const copy = structuredClone(c);
  saveChain = saveChain
    .catch(() => {})
    .then(() => api("/api/collections", copy));
  saveChain.catch((e) => toast(csText("保存に失敗しました: {0}", e.message)));
  return saveChain;
}
