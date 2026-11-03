# PadBridge

Usa tus controles de **PlayStation 5 (DualSense y DualSense Edge)** en cualquier juego de PC: cada control aparece como un **control de Xbox 360 virtual**, y PadBridge le da acceso a lo propio del DualSense: **gatillos adaptativos**, **barra de luz**, **LEDs**, **vibración** y **sensibilidad por control**.

Pensado para jugar en pantalla dividida con varios controles en el mismo PC, por ejemplo Halo: The Master Chief Collection con [AlphaRing](https://github.com/megabitt01/AlphaRing).

## Funciones

- **Varios controles a la vez**, por USB o Bluetooth. Cada uno es un control de Xbox aparte (XInput admite hasta 4 en total, contando los de Xbox reales).
- **Gatillos adaptativos** en L2 y R2, en cualquier juego: resistencia, resistencia progresiva, arma (gatillo con clic), vibración, arco, galope, ametralladora y fuerza por zonas. Trae preajustes (Rifle, Pistola, Escopeta, Arco, Carreras…).
- **Sensibilidad por control**: sensibilidad, curva de respuesta, zona muerta, anti-zona muerta e invertir ejes de cada stick; zona muerta y punto máximo de cada gatillo (disparo rápido). Con gráfica de la curva y vista en vivo.
- **Luces**: color de la barra de luz, brillo, modo arcoíris o nivel de batería; LEDs de jugador con el número de control Xbox o un patrón propio; LED del micrófono.
- **Vibración** del juego pasada al DualSense, con intensidad ajustable y botones de prueba.
- **Panel táctil** como mouse, o su clic como botón Back.
- **Vista en vivo**: botones, sticks (lo que mueves y lo que recibe el juego), gatillos, panel táctil, giroscopio, acelerómetro y batería.
- **Controles de Xbox** (USB, Bluetooth o adaptador): aparecen en la misma lista, con su número de control, su batería (si la informan), vista en vivo, prueba de vibración e Identificar. Los juegos los leen directamente, sin pasar por PadBridge.
- **Perfiles por control**, guardados solos; se reconocen por su dirección MAC, también si cambias de USB a Bluetooth. "Aplicar a todos" copia uno a los demás.
- Sigue en la **bandeja del sistema** al cerrar la ventana, para que los controles sigan funcionando.

## Requisitos

- Windows 10 u 11.
- El driver **[ViGEmBus](https://github.com/nefarius/ViGEmBus/releases)** de Nefarius (crea los controles de Xbox virtuales). Si ya usaste DS4Windows o DualSenseX, probablemente ya lo tienes.
- Cerrar **DualSenseX, DSX o DS4Windows** mientras usas PadBridge: si no, cada control aparece dos veces. PadBridge avisa si alguno está abierto.

## Uso

1. Instala PadBridge y ábrelo.
2. Conecta tus DualSense por USB o Bluetooth: aparecen solos en la lista, con el número de control Xbox que les tocó (también en sus LEDs).
3. Abre el juego después de conectar los controles: muchos juegos solo buscan controles al arrancar.

El botón **Identificar** hace parpadear y vibrar el control seleccionado, para saber cuál es cuál.

## Compilar

Necesitas [Rust](https://rustup.rs) (toolchain MSVC), [Node.js](https://nodejs.org) 20+ y Visual Studio 2022 Build Tools (C++).

```
npm install
npm run tauri dev     # desarrollo
npm run tauri build   # instalador en src-tauri/target/release/bundle/nsis
```

Pruebas del protocolo y de las curvas: `cargo test` dentro de `src-tauri`.

## Cómo está hecho

- **Backend en Rust** ([Tauri 2](https://tauri.app)): un hilo por control lee sus reportes HID ([hidapi](https://github.com/ruabmbua/hidapi-rs)), alimenta su control virtual ([vigem-client](https://github.com/CasualX/vigem-client)) y escribe luces, vibración y gatillos. El protocolo del DualSense (USB y Bluetooth, con su CRC32) está en `src-tauri/src/ds/protocol.rs`.
- **Interfaz en Svelte 5 + TypeScript**, sin librerías de UI.

Los formatos de los reportes siguen el driver `hid-playstation` de Linux, y las codificaciones de los efectos de gatillo, las notas públicas de Nielk1.

## Licencia

[MIT](LICENSE).
