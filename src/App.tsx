import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// Struktur ini HARUS sama persis dengan struct AppConfig di main.rs (Rust),
// supaya data yang disimpan/dibaca tidak salah bentuk.
interface AppConfig {
  threads_user_id: string;
  threads_access_token: string;
  cloudinary: {
    cloud_name: string;
    api_key: string;
    api_secret: string;
  };
  gemini_api_keys: string[];
  ai_reply_enabled: boolean;
  queue_folder: string;
  posted_folder: string;
}

const EMPTY_CONFIG: AppConfig = {
  threads_user_id: "",
  threads_access_token: "",
  cloudinary: { cloud_name: "", api_key: "", api_secret: "" },
  gemini_api_keys: [""],
  ai_reply_enabled: false,
  queue_folder: "",
  posted_folder: "",
};

export default function App() {
  const [config, setConfig] = useState<AppConfig>(EMPTY_CONFIG);
  const [status, setStatus] = useState<string>("");
  const [loading, setLoading] = useState(true);

  // Muat config yang sudah tersimpan sebelumnya (kalau ada) saat app dibuka
  useEffect(() => {
    invoke<AppConfig>("load_config")
      .then((loaded) => {
        // Jaga-jaga kalau gemini_api_keys kosong, selalu ada minimal 1 kotak input
        if (!loaded.gemini_api_keys || loaded.gemini_api_keys.length === 0) {
          loaded.gemini_api_keys = [""];
        }
        setConfig(loaded);
      })
      .catch((err) => setStatus(`Gagal memuat pengaturan tersimpan: ${err}`))
      .finally(() => setLoading(false));
  }, []);

  async function handleSave() {
    setStatus("Menyimpan...");
    try {
      // Buang baris Gemini key yang kosong sebelum disimpan
      const cleaned = {
        ...config,
        gemini_api_keys: config.gemini_api_keys.filter((k) => k.trim() !== ""),
      };
      await invoke("save_config", { config: cleaned });
      setStatus("Tersimpan.");
    } catch (err) {
      setStatus(`Gagal menyimpan: ${err}`);
    }
  }

  async function pickFolder(target: "queue_folder" | "posted_folder") {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setConfig((prev) => ({ ...prev, [target]: selected }));
    }
  }

  function updateGeminiKey(index: number, value: string) {
    setConfig((prev) => {
      const updated = [...prev.gemini_api_keys];
      updated[index] = value;
      return { ...prev, gemini_api_keys: updated };
    });
  }

  function addGeminiKeyField() {
    setConfig((prev) => ({
      ...prev,
      gemini_api_keys: [...prev.gemini_api_keys, ""],
    }));
  }

  function removeGeminiKeyField(index: number) {
    setConfig((prev) => {
      const updated = prev.gemini_api_keys.filter((_, i) => i !== index);
      return { ...prev, gemini_api_keys: updated.length > 0 ? updated : [""] };
    });
  }

  if (loading) return <div className="container">Memuat pengaturan...</div>;

  return (
    <div className="container">
      <h1>Threads Automator</h1>
      <p className="subtitle">Pengaturan kredensial &amp; perilaku sistem</p>

      {/* ===== BAGIAN THREADS ===== */}
      <section>
        <h2>Threads API</h2>

        <label className="field-label">
          Threads User ID
          <span className="hint">ID akun Threads kamu (dari endpoint /me)</span>
        </label>
        <input
          type="text"
          placeholder="Contoh: 17841400..."
          value={config.threads_user_id}
          onChange={(e) => setConfig({ ...config, threads_user_id: e.target.value })}
        />

        <label className="field-label">
          Threads Access Token
          <span className="hint">Long-lived token, direfresh otomatis tiap minggu</span>
        </label>
        <input
          type="password"
          placeholder="Token panjang dari Meta Developer"
          value={config.threads_access_token}
          onChange={(e) => setConfig({ ...config, threads_access_token: e.target.value })}
        />
      </section>

      {/* ===== BAGIAN CLOUDINARY ===== */}
      <section>
        <h2>Cloudinary (hosting video/gambar)</h2>

        <label className="field-label">
          Cloudinary Cloud Name
          <span className="hint">Dari dashboard cloudinary.com</span>
        </label>
        <input
          type="text"
          placeholder="Contoh: dxyz1234"
          value={config.cloudinary.cloud_name}
          onChange={(e) =>
            setConfig({ ...config, cloudinary: { ...config.cloudinary, cloud_name: e.target.value } })
          }
        />

        <label className="field-label">
          Cloudinary API Key
          <span className="hint">Angka di dashboard Cloudinary, bukan API Secret</span>
        </label>
        <input
          type="text"
          placeholder="Contoh: 123456789012345"
          value={config.cloudinary.api_key}
          onChange={(e) =>
            setConfig({ ...config, cloudinary: { ...config.cloudinary, api_key: e.target.value } })
          }
        />

        <label className="field-label">
          Cloudinary API Secret
          <span className="hint">Jaga kerahasiaannya, jangan dibagikan</span>
        </label>
        <input
          type="password"
          placeholder="API Secret Cloudinary"
          value={config.cloudinary.api_secret}
          onChange={(e) =>
            setConfig({ ...config, cloudinary: { ...config.cloudinary, api_secret: e.target.value } })
          }
        />
      </section>

      {/* ===== BAGIAN GEMINI (MULTI KEY) ===== */}
      <section>
        <h2>Gemini API Key (untuk balasan AI)</h2>
        <p className="hint">
          Bisa isi lebih dari satu — kalau key pertama kuotanya habis, sistem otomatis
          coba key berikutnya secara berurutan.
        </p>

        {config.gemini_api_keys.map((key, index) => (
          <div className="gemini-key-row" key={index}>
            <label className="field-label">
              Gemini API Key #{index + 1}
              <span className="hint">
                {index === 0 ? "Dicoba pertama kali" : `Cadangan urutan ke-${index + 1}`}
              </span>
            </label>
            <div className="key-input-group">
              <input
                type="password"
                placeholder={`Tempel Gemini API Key ke-${index + 1} di sini`}
                value={key}
                onChange={(e) => updateGeminiKey(index, e.target.value)}
              />
              {config.gemini_api_keys.length > 1 && (
                <button
                  type="button"
                  className="btn-remove"
                  onClick={() => removeGeminiKeyField(index)}
                  title="Hapus key ini"
                >
                  ✕
                </button>
              )}
            </div>
          </div>
        ))}

        <button type="button" className="btn-add" onClick={addGeminiKeyField}>
          + Tambah Gemini API Key
        </button>
      </section>

      {/* ===== TOGGLE AI REPLY ===== */}
      <section>
        <h2>Balasan Komentar Otomatis (AI)</h2>
        <label className="toggle-row">
          <input
            type="checkbox"
            checked={config.ai_reply_enabled}
            onChange={(e) => setConfig({ ...config, ai_reply_enabled: e.target.checked })}
          />
          <span>
            {config.ai_reply_enabled
              ? "AKTIF — sistem akan membalas komentar otomatis"
              : "NONAKTIF — komentar tidak akan dibalas otomatis"}
          </span>
        </label>
      </section>

      {/* ===== FOLDER ===== */}
      <section>
        <h2>Folder Konten</h2>

        <label className="field-label">Folder Queue (stok konten baru)</label>
        <div className="folder-picker-row">
          <input type="text" readOnly value={config.queue_folder} placeholder="Belum dipilih" />
          <button type="button" onClick={() => pickFolder("queue_folder")}>
            Pilih Folder
          </button>
        </div>

        <label className="field-label">Folder Posted (arsip yang sudah tayang)</label>
        <div className="folder-picker-row">
          <input type="text" readOnly value={config.posted_folder} placeholder="Belum dipilih" />
          <button type="button" onClick={() => pickFolder("posted_folder")}>
            Pilih Folder
          </button>
        </div>
      </section>

      <button type="button" className="btn-save" onClick={handleSave}>
        Simpan Pengaturan
      </button>

      {status && <p className="status">{status}</p>}
    </div>
  );
}
