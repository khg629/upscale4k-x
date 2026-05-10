# UpScale4K (Cross-platform)

> macOS·Windows·Linux에서 동일하게 동작하는 로컬 이미지 4× 업스케일 앱.
> [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN) + [ncnn-vulkan](https://github.com/Tencent/ncnn) 기반, [Tauri 2](https://tauri.app) + Svelte로 구현.

> 🔁 **macOS 전용 SwiftUI 버전**은 [`khg629/upscale4k`](https://github.com/khg629/upscale4k)에 별도로 보존되어 있습니다 (v0.1.1). 이 저장소는 그 후속·통합 버전입니다.

---

## 주요 기능

- 🔒 **100% 로컬 처리** — 외부 네트워크 호출 없음
- 🖥️ **Windows·macOS 동일 UI** — 강의·자료에서 한 화면만 보여줘도 됨
- 📦 **배치 처리** — 한 번에 100장 이상 (드래그 앤 드롭, 폴더 드롭 지원)
- 🤖 **모델 2종** — General(사진), Anime(일러스트·텍스트)
- 🔢 **배율** — 2× / 3× / 4× (2·3×는 항상 4×로 추론 후 Lanczos3 다운스케일 → 픽셀 깨짐 없음)
- 📁 **출력 폴더 지정** — 완료 시 시스템 파일 탐색기에서 자동 열림
- ⏹ **취소 가능** — 큐 개별 ✕ / 처리 중 일괄 취소 (Esc)
- ⚡ **GPU 가속** — Vulkan(Windows/Linux) · Metal-via-MoltenVK(macOS)

---

## 시스템 요구사항

| OS | 요구 |
| --- | --- |
| **macOS** | 13 Ventura 이상, Apple Silicon (M1~M4) 권장 |
| **Windows** | 10 이상, Vulkan 지원 GPU (대부분의 NVIDIA·AMD·Intel iGPU) |
| **Linux** | (실험적) Vulkan 지원 환경 |

---

## 다운로드 / 설치

### macOS

1. [Releases](https://github.com/khg629/upscale4k-x/releases/latest)에서 `UpScale4K_x.y.z_aarch64.dmg` 받기
2. 더블클릭 → **UpScale4K**를 **Applications** 폴더로 드래그
3. **첫 실행 (한 번만)**: Applications 폴더에서 우클릭 → "열기" → 경고창의 "열기" 클릭
4. 이후로는 그냥 더블클릭

> ℹ️ 본 앱은 Apple Developer 공증을 받지 않은 무료 배포본이라 첫 실행 시 위 절차가 필요합니다. 코드는 100% 오픈소스입니다.

### Windows

1. [Releases](https://github.com/khg629/upscale4k-x/releases/latest)에서 `UpScale4K_x.y.z_x64-setup.exe` 또는 `UpScale4K_x.y.z_x64_en-US.msi` 받기
2. 실행 → **"Windows에서 PC를 보호했습니다"** 경고가 뜨면 **"추가 정보" → "실행"** 클릭
3. 설치 마법사 진행

> ℹ️ Windows SmartScreen 경고 역시 코드 사이닝 인증서 미적용 때문입니다.

---

## 사용법

1. 앱 실행 → 가운데 큐 영역에 이미지/폴더 끌어다 놓기 (또는 "파일 추가" / "폴더 추가" 버튼)
2. 모델 선택: **General** (사진) 또는 **Anime** (일러스트·텍스트·인포그래픽 추천)
3. 배율 선택: 2× / 3× / 4×
4. (선택) 출력 폴더 지정 — 안 하면 입력 파일 옆에 저장
5. **Enter** 또는 "업스케일 시작" 클릭
6. 처리 중 **Esc**로 취소 가능. 큐의 각 항목은 ✕로 시작 전에 제거 가능

### 모델 선택 가이드

| 콘텐츠 종류 | 권장 모델 |
| --- | --- |
| 인물·풍경·자연 사진 | General |
| 일러스트, 만화, 인포그래픽, **강의 슬라이드** | **Anime** (텍스트·라인 보존 우수) |
| 사진 + 사진 안의 작은 텍스트 | Anime이 그나마 나음. 단일 모델로 완벽한 결과는 어려움 |

---

## 소스에서 빌드

### 사전 요구

- **Rust 1.80+**: [rustup.rs](https://rustup.rs)
- **Node.js 20+**: [nodejs.org](https://nodejs.org)
- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Windows**: Visual Studio 2019/2022 Build Tools + WebView2
- **Linux**: WebKitGTK 4.1, libsoup3 (`sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev`)

### 외부 자원 다운로드 (모델·바이너리)

저장소에는 ncnn-vulkan 바이너리와 모델 파일이 포함되어 있지 않습니다 (라이선스·용량 사정).

```bash
mkdir -p src-tauri/binaries src-tauri/resources/models src-tauri/binaries/win-deps

# macOS arm64
curl -L https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-macos.zip \
    -o /tmp/macos.zip
unzip -o /tmp/macos.zip -d /tmp/macos
cp /tmp/macos/realesrgan-ncnn-vulkan src-tauri/binaries/realesrgan-aarch64-apple-darwin
chmod +x src-tauri/binaries/realesrgan-aarch64-apple-darwin
cp -R /tmp/macos/models/* src-tauri/resources/models/
xattr -dr com.apple.quarantine src-tauri/binaries 2>/dev/null || true

# Windows x64
curl -L https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-windows.zip \
    -o /tmp/windows.zip
unzip -o /tmp/windows.zip -d /tmp/windows
cp /tmp/windows/realesrgan-ncnn-vulkan.exe src-tauri/binaries/realesrgan-x86_64-pc-windows-msvc.exe
cp /tmp/windows/vcomp140.dll src-tauri/binaries/win-deps/
cp /tmp/windows/vcomp140d.dll src-tauri/binaries/win-deps/

rm -rf /tmp/macos /tmp/windows /tmp/macos.zip /tmp/windows.zip
```

### 빌드

```bash
git clone https://github.com/khg629/upscale4k-x.git
cd upscale4k-x
npm install

# 개발 모드 (hot reload)
npm run tauri dev

# 정식 빌드 (.app, .dmg 또는 .exe, .msi 생성)
npm run tauri build
```

빌드 산출물 위치:
- macOS: `src-tauri/target/release/bundle/macos/UpScale4K.app`, `src-tauri/target/release/bundle/dmg/*.dmg`
- Windows: `src-tauri/target/release/bundle/nsis/*.exe`, `src-tauri/target/release/bundle/msi/*.msi`

---

## 라이선스

본 저장소의 Rust·TypeScript·Svelte 코드는 **MIT License** 하에 배포됩니다 ([LICENSE](LICENSE)).

번들된 ncnn-vulkan 바이너리와 사전 학습 모델은 **BSD 3-Clause** 라이선스로, [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)를 참고하세요.

---

## 감사의 글

- **[Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN)** (Xintao Wang 외, Tencent ARC Lab) — 핵심 업스케일 모델
- **[ncnn](https://github.com/Tencent/ncnn)** (Tencent) — Vulkan 기반 추론 엔진
- **[Tauri](https://tauri.app)** — 크로스 플랫폼 데스크탑 프레임워크
- **[Svelte](https://svelte.dev)** — UI 프레임워크

---

## 기여 / 이슈

버그 제보·기능 제안은 [Issues](https://github.com/khg629/upscale4k-x/issues)에 부탁드립니다.
