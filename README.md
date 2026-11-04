# PadBridge

App de Windows para usar varios mandos en el mismo PC. Cada **DualSense**, **DualSense Edge** o **Xbox** elige cómo lo ve el juego:

- **Nativo**: el juego habla con el mando real. Un DualSense sigue siendo DualSense (gatillos adaptativos del juego, hápticos, giroscopio). Un Xbox sigue siendo Xbox.
- **Xbox 360 virtual**: el juego lo ve como un control de Xbox 360, mediante [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases).
- **DualShock 4 virtual**: el juego lo ve como un mando de PlayStation. No es un DualSense virtual: los juegos que exigen el mando real no reciben gatillos adaptativos por este camino.

El mando físico no se oculta. Si hay un virtual, el juego puede ver los dos.

Pensado para jugar en pantalla dividida, por ejemplo Halo: The Master Chief Collection con [AlphaRing](https://github.com/megabitt01/AlphaRing).

## Funciones

- **Varios controles a la vez**, por USB o Bluetooth. XInput admite hasta 4 controles de Xbox en total, entre reales y virtuales. Un DualShock 4 virtual no ocupa uno de esos huecos.
- **Gatillos adaptativos** en el DualSense, puestos por PadBridge: resistencia, progresiva, arma, vibración, arco, galope, ametralladora y zonas. En modo nativo no se escriben, para que el juego mande en los gatillos y las luces.
- **Sensibilidad por control** en el mando virtual: curva, zona muerta, anti-zona muerta, invertir ejes, zona muerta y punto máximo de cada gatillo.
- **Giroscopio** del DualSense como mouse, o sumado al stick derecho del control virtual.
- **Remapeo de botones**, incluidas las paletas y los botones FN del DualSense Edge. Solo afecta al control virtual.
- **Luces**: barra de luz, LEDs de jugador y LED del micrófono, cuando PadBridge escribe al DualSense.
- **Vibración** del juego en modo Xbox 360, con intensidad y botones de prueba.
- **Panel táctil** como mouse, o su clic como Back / Share.
- **Vista en vivo** con la silueta del DualSense o la del Xbox, según el mando.
- **Perfiles por mando** (por dirección MAC) y **perfiles por juego**: cuando ese `.exe` está en primer plano se usa su perfil. Mientras la ventana de PadBridge está delante se mantiene el último juego.
- **Arranque con Windows**, bandeja del sistema e inicio minimizado.

## Requisitos

- Windows 10 u 11.
- El driver **[ViGEmBus](https://github.com/nefarius/ViGEmBus/releases)** para los controles virtuales. Si ya usaste DS4Windows o DualSenseX, probablemente ya lo tienes.
- Cerrar **DualSenseX, DSX o DS4Windows** mientras usas PadBridge. La app avisa si alguno está abierto.

## Uso

1. Compila o instala PadBridge y ábrelo.
2. Conecta los mandos. Elige la salida en la pestaña **Salida**.
3. Abre el juego después de conectar los controles: muchos juegos solo buscan controles al arrancar.

**Identificar** hace parpadear y vibrar el control seleccionado.

## Compilar

Hace falta [Rust](https://rustup.rs) con el toolchain MSVC y Visual Studio 2022 Build Tools (C++). La interfaz es [Slint](https://slint.dev/) y no usa Node ni WebView.

```
cd src-tauri
cargo run        # desarrollo
cargo build --release
```

El ejecutable queda en `src-tauri/target/release/padbridge.exe`.

Pruebas del protocolo, de las curvas y de la migración de perfiles: `cargo test` dentro de `src-tauri`.

Por defecto el renderer es FemtoVG sobre Direct3D (`SLINT_BACKEND=winit-femtovg-wgpu`). Para forzar el renderer de software, más pequeño y sin GPU: `SLINT_BACKEND=winit-software`.

## Cómo está hecho

- **Un solo binario en Rust.** Un hilo por DualSense lee sus reportes HID ([hidapi](https://github.com/ruabmbua/hidapi-rs)), alimenta el control virtual ([vigem-client](https://github.com/CasualX/vigem-client)) y, fuera del modo nativo, escribe luces, vibración y gatillos. El protocolo está en `src-tauri/src/ds/protocol.rs`.
- **Interfaz en Slint**, sin WebView. El estilo es Fluent oscuro.

Los formatos de los reportes siguen el driver `hid-playstation` de Linux, y las codificaciones de los efectos de gatillo, las notas públicas de Nielk1.

La interfaz usa Slint bajo su licencia royalty-free para aplicaciones de escritorio. El código de PadBridge sigue siendo MIT. La pantalla de ajustes muestra el crédito de Slint.

## Licencia

[MIT](LICENSE).
