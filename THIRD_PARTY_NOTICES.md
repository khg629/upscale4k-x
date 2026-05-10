# Third-Party Notices

UpScale4K (cross-platform)는 다음 오픈소스 컴포넌트를 사용·재배포합니다.

---

## 1. Real-ESRGAN

- **프로젝트**: https://github.com/xinntao/Real-ESRGAN
- **저작자**: Xintao Wang, Liangbin Xie, Chao Dong, Ying Shan (Tencent ARC Lab)
- **라이선스**: BSD 3-Clause License
- **사용 방식**: `realesrgan-ncnn-vulkan` 사전 빌드 바이너리(macOS arm64 / Windows x64) 및 사전 학습 모델 가중치를 본 앱의 번들·인스톨러 내부에 동봉
- **인용 (논문)**:
  > Wang, X., Xie, L., Dong, C., & Shan, Y. (2021). *Real-ESRGAN: Training Real-World Blind Super-Resolution with Pure Synthetic Data.* In Proceedings of the IEEE/CVF International Conference on Computer Vision Workshops.

```
BSD 3-Clause License

Copyright (c) 2021, Xintao Wang
All rights reserved.

(전체 라이선스 원문: https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE)
```

---

## 2. ncnn / realesrgan-ncnn-vulkan

`realesrgan-ncnn-vulkan` 바이너리는 [ncnn](https://github.com/Tencent/ncnn) 추론 프레임워크 기반입니다.

- **프로젝트**: https://github.com/Tencent/ncnn
- **저작자**: Tencent
- **라이선스**: BSD 3-Clause License
- **사용 방식**: 사전 빌드 바이너리(macOS arm64 / Windows x64)를 동봉

---

## 3. 사전 학습 모델 가중치

| 파일 | 용도 | 출처 |
| --- | --- | --- |
| `realesrgan-x4plus.{bin,param}` | 일반 사진 4× | Real-ESRGAN 공식 릴리즈 |
| `realesrgan-x4plus-anime.{bin,param}` | 일러스트·애니 4× | Real-ESRGAN 공식 릴리즈 |
| `realesr-animevideov3-x{2,3,4}.{bin,param}` | 애니 비디오 프레임 | Real-ESRGAN 공식 릴리즈 |

위 모델 가중치는 Real-ESRGAN 프로젝트의 공식 릴리즈에서 받은 그대로 동봉됩니다.

---

## 4. Tauri 및 의존성

[Tauri 2](https://tauri.app) (MIT/Apache-2.0) 및 그 Rust·JS 의존성 다수가 포함됩니다. 전체 라이선스 정보는 빌드 산출물의 `cargo about` 또는 npm `license-checker`로 확인할 수 있습니다. 모든 의존성은 MIT, Apache-2.0, BSD, ISC 등 OSI 승인 라이선스 하에 있습니다.

---

## 5. Microsoft Visual C++ Redistributable

Windows 빌드는 `vcomp140.dll`(Visual C++ OpenMP 런타임)을 동봉합니다 — Microsoft의 [Microsoft Software License Terms (Visual Studio Redistributables)](https://docs.microsoft.com/en-us/visualstudio/releases/2019/redistribution) 하에 재배포가 허용됩니다.

---

## 6. 본 저장소 코드 (UpScale4K cross-platform)

본 저장소의 Rust·TypeScript·Svelte 소스 코드와 빌드 스크립트는 별도 명시가 없는 한 **MIT License**(루트의 `LICENSE` 파일) 하에 배포됩니다.

---

## 변경 사항

본 프로젝트는 Real-ESRGAN, ncnn, ncnn-vulkan 바이너리 및 모델 파일에 어떠한 코드 수준의 변경도 가하지 않으며, 공식 릴리즈에서 배포된 그대로를 재배포합니다. 따라서 위 BSD 3-Clause 조건의 "Redistributions in binary form" 요구만 충족하면 됩니다 — 본 문서가 그 고지를 수행합니다.
