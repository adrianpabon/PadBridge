# PadBridge

Convierte cada control de PlayStation conectado al PC (DualSense, DualShock 4) en su propio control de **Xbox 360 virtual**, para juegos que solo entienden controles de Xbox (XInput), como Halo: The Master Chief Collection con [AlphaRing](https://github.com/megabitt01/AlphaRing).

- Varios controles a la vez, por USB o Bluetooth (XInput admite hasta 4 controles en total, contando los de Xbox).
- Botones, sticks, gatillos y vibración.
- Las luces de jugador del control muestran qué número de control Xbox le tocó.

## Requisitos

- Windows 10/11.
- El driver [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) de Nefarius (lo instalan también DualSenseX/DSX y DS4Windows).
- Cerrar DualSenseX, DSX o DS4Windows mientras se usa PadBridge; si no, cada control aparece dos veces.

## Uso

1. Conecta los controles de PlayStation.
2. Abre `PadBridge.exe` y déjalo abierto mientras juegas.
3. Abre el juego después, para que encuentre todos los controles.

Para salir, cierra la ventana o presiona Ctrl+C: los controles virtuales se quitan.

## Compilar

Visual Studio 2022 (C++), CMake 3.21+ y [vcpkg](https://github.com/microsoft/vcpkg):

```
cmake -S . -B build -G "Visual Studio 17 2022" -A x64 -DCMAKE_TOOLCHAIN_FILE=<vcpkg>/scripts/buildsystems/vcpkg.cmake
cmake --build build --config Release
```

Sale un solo `build/Release/PadBridge.exe`, sin DLLs aparte.

Usa [SDL2](https://www.libsdl.org/) para leer los controles y [ViGEmClient](https://github.com/nefarius/ViGEmClient) (MIT) para crear los controles virtuales.
