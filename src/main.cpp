// PadBridge: every PlayStation controller (DualSense, DualShock 4) connected to the PC shows up as its own virtual
// Xbox 360 controller, so XInput-only games (Halo MCC, ...) see one more pad per PlayStation controller.
//
// Reading: SDL2's HIDAPI drivers (USB and Bluetooth). Virtual pads: the ViGEmBus driver through ViGEmClient.
// Rumble the game sends to a virtual pad goes back to its PlayStation controller, and the controller's player
// LEDs show which XInput slot it got.

#include <Windows.h>
#include <TlHelp32.h>

#include <ViGEm/Client.h>

#define SDL_MAIN_HANDLED
#include <SDL.h>

#include <algorithm>
#include <atomic>
#include <cstdio>
#include <cstring>
#include <map>
#include <memory>

namespace {
    constexpr Uint16 kSonyVendor = 0x054C;

    std::atomic<bool> g_running{true};
    PVIGEM_CLIENT g_vigem = nullptr;

    struct Pad {
        SDL_GameController* controller = nullptr;
        PVIGEM_TARGET target = nullptr;
        XUSB_REPORT last{};
        bool sent = false;
        int slot = -1;  // XInput user index, once the bus has given one
        // Written by ViGEm's notification thread, read by the main loop.
        std::atomic<UCHAR> large{0}, small{0};
        UCHAR appliedLarge = 0, appliedSmall = 0;
        Uint32 rumbleRefresh = 0;
    };

    std::map<SDL_JoystickID, std::unique_ptr<Pad>> g_pads;

    std::atomic<bool> g_cleanedUp{false};

    BOOL WINAPI OnConsoleCtrl(DWORD type) {
        g_running = false;
        // Closing the window ends the process once this returns: give the main loop time to remove the virtual pads.
        if (type == CTRL_CLOSE_EVENT || type == CTRL_LOGOFF_EVENT || type == CTRL_SHUTDOWN_EVENT)
            for (int i = 0; i < 300 && !g_cleanedUp; ++i) Sleep(10);
        return TRUE;
    }

    void CALLBACK OnX360Notification(PVIGEM_CLIENT, PVIGEM_TARGET, UCHAR large, UCHAR small, UCHAR, LPVOID user) {
        auto* pad = static_cast<Pad*>(user);
        pad->large = large;
        pad->small = small;
    }

    bool ProcessRunning(const wchar_t* exe) {
        HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if (snapshot == INVALID_HANDLE_VALUE) return false;
        PROCESSENTRY32W entry{sizeof(entry)};
        bool found = false;
        for (BOOL ok = Process32FirstW(snapshot, &entry); ok && !found; ok = Process32NextW(snapshot, &entry))
            found = _wcsicmp(entry.szExeFile, exe) == 0;
        CloseHandle(snapshot);
        return found;
    }

    SHORT InvertAxis(Sint16 value) {
        // SDL's Y axes grow downwards, XInput's upwards; -(-32768) doesn't fit in a SHORT.
        return value == SDL_JOYSTICK_AXIS_MIN ? SHRT_MAX : static_cast<SHORT>(-value);
    }

    BYTE Trigger(Sint16 value) {
        return static_cast<BYTE>(std::clamp<int>(value, 0, SDL_JOYSTICK_AXIS_MAX) * 255 / SDL_JOYSTICK_AXIS_MAX);
    }

    XUSB_REPORT ReadReport(SDL_GameController* c) {
        static const struct { SDL_GameControllerButton from; USHORT to; } kButtons[] = {
            {SDL_CONTROLLER_BUTTON_A, XUSB_GAMEPAD_A},
            {SDL_CONTROLLER_BUTTON_B, XUSB_GAMEPAD_B},
            {SDL_CONTROLLER_BUTTON_X, XUSB_GAMEPAD_X},
            {SDL_CONTROLLER_BUTTON_Y, XUSB_GAMEPAD_Y},
            {SDL_CONTROLLER_BUTTON_BACK, XUSB_GAMEPAD_BACK},       // Create / Share
            {SDL_CONTROLLER_BUTTON_TOUCHPAD, XUSB_GAMEPAD_BACK},   // touchpad click: Back too (scoreboard)
            {SDL_CONTROLLER_BUTTON_START, XUSB_GAMEPAD_START},     // Options
            {SDL_CONTROLLER_BUTTON_GUIDE, XUSB_GAMEPAD_GUIDE},     // PS
            {SDL_CONTROLLER_BUTTON_LEFTSTICK, XUSB_GAMEPAD_LEFT_THUMB},
            {SDL_CONTROLLER_BUTTON_RIGHTSTICK, XUSB_GAMEPAD_RIGHT_THUMB},
            {SDL_CONTROLLER_BUTTON_LEFTSHOULDER, XUSB_GAMEPAD_LEFT_SHOULDER},
            {SDL_CONTROLLER_BUTTON_RIGHTSHOULDER, XUSB_GAMEPAD_RIGHT_SHOULDER},
            {SDL_CONTROLLER_BUTTON_DPAD_UP, XUSB_GAMEPAD_DPAD_UP},
            {SDL_CONTROLLER_BUTTON_DPAD_DOWN, XUSB_GAMEPAD_DPAD_DOWN},
            {SDL_CONTROLLER_BUTTON_DPAD_LEFT, XUSB_GAMEPAD_DPAD_LEFT},
            {SDL_CONTROLLER_BUTTON_DPAD_RIGHT, XUSB_GAMEPAD_DPAD_RIGHT},
        };

        XUSB_REPORT report{};
        for (const auto& b : kButtons)
            if (SDL_GameControllerGetButton(c, b.from))
                report.wButtons |= b.to;
        report.bLeftTrigger = Trigger(SDL_GameControllerGetAxis(c, SDL_CONTROLLER_AXIS_TRIGGERLEFT));
        report.bRightTrigger = Trigger(SDL_GameControllerGetAxis(c, SDL_CONTROLLER_AXIS_TRIGGERRIGHT));
        report.sThumbLX = SDL_GameControllerGetAxis(c, SDL_CONTROLLER_AXIS_LEFTX);
        report.sThumbLY = InvertAxis(SDL_GameControllerGetAxis(c, SDL_CONTROLLER_AXIS_LEFTY));
        report.sThumbRX = SDL_GameControllerGetAxis(c, SDL_CONTROLLER_AXIS_RIGHTX);
        report.sThumbRY = InvertAxis(SDL_GameControllerGetAxis(c, SDL_CONTROLLER_AXIS_RIGHTY));
        return report;
    }

    void AddPad(int deviceIndex) {
        if (SDL_JoystickGetDeviceVendor(deviceIndex) != kSonyVendor)
            return; // Xbox pads (and our own virtual ones) are already XInput
        SDL_GameController* controller = SDL_GameControllerOpen(deviceIndex);
        if (!controller) {
            std::printf("  ! no se pudo abrir el control %d: %s\n", deviceIndex, SDL_GetError());
            return;
        }
        SDL_JoystickID id = SDL_JoystickInstanceID(SDL_GameControllerGetJoystick(controller));
        if (g_pads.count(id)) { SDL_GameControllerClose(controller); return; }

        auto pad = std::make_unique<Pad>();
        pad->controller = controller;
        pad->target = vigem_target_x360_alloc();
        VIGEM_ERROR error = vigem_target_add(g_vigem, pad->target);
        if (!VIGEM_SUCCESS(error)) {
            std::printf("  ! no se pudo crear el control virtual (error 0x%X)\n", error);
            vigem_target_free(pad->target);
            SDL_GameControllerClose(controller);
            return;
        }
        vigem_target_x360_register_notification(g_vigem, pad->target, &OnX360Notification, pad.get());

        const char* name = SDL_GameControllerName(controller);
        std::printf("+ %s conectado -> control Xbox 360 virtual creado\n", name ? name : "Control PlayStation");
        g_pads[id] = std::move(pad);
    }

    void RemovePad(SDL_JoystickID id) {
        auto it = g_pads.find(id);
        if (it == g_pads.end()) return;
        Pad& pad = *it->second;
        vigem_target_x360_unregister_notification(pad.target);
        vigem_target_remove(g_vigem, pad.target);
        vigem_target_free(pad.target);
        SDL_GameControllerClose(pad.controller);
        if (pad.slot >= 0)
            std::printf("- control desconectado (era el control Xbox %d)\n", pad.slot + 1);
        else
            std::printf("- control desconectado\n");
        g_pads.erase(it);
    }

    void UpdatePad(Pad& pad) {
        if (pad.slot < 0) {
            ULONG index;
            if (VIGEM_SUCCESS(vigem_target_x360_get_user_index(g_vigem, pad.target, &index))) {
                pad.slot = static_cast<int>(index);
                SDL_GameControllerSetPlayerIndex(pad.controller, pad.slot); // lights the matching player LEDs
                std::printf("  = es el control Xbox %d (ranura XInput %d)\n", pad.slot + 1, pad.slot);
            }
        }

        XUSB_REPORT report = ReadReport(pad.controller);
        if (!pad.sent || std::memcmp(&report, &pad.last, sizeof(report)) != 0) {
            vigem_target_x360_update(g_vigem, pad.target, report);
            pad.last = report;
            pad.sent = true;
        }

        // Rumble: the game sets a level until it changes it; SDL's needs a duration, so it's renewed while on.
        UCHAR large = pad.large, small = pad.small;
        Uint32 now = SDL_GetTicks();
        bool changed = large != pad.appliedLarge || small != pad.appliedSmall;
        if (changed || ((large || small) && SDL_TICKS_PASSED(now, pad.rumbleRefresh))) {
            SDL_GameControllerRumble(pad.controller, large * 257, small * 257, (large || small) ? 1000 : 0);
            pad.appliedLarge = large;
            pad.appliedSmall = small;
            pad.rumbleRefresh = now + 500;
        }
    }
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0); // show each line at once, also when redirected
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleTitleW(L"PadBridge");
    SetConsoleCtrlHandler(&OnConsoleCtrl, TRUE);

    std::printf("PadBridge - controles PlayStation como controles Xbox\n"
                "Deja esta ventana abierta mientras juegas. Ctrl+C o cerrar la ventana para salir.\n\n");

    if (ProcessRunning(L"DualSenseX.exe") || ProcessRunning(L"DSX.exe") || ProcessRunning(L"DS4Windows.exe"))
        std::printf("AVISO: DualSenseX/DSX/DS4Windows esta abierto. Cierralo, o cada control aparecera dos veces.\n\n");

    g_vigem = vigem_alloc();
    VIGEM_ERROR error = g_vigem ? vigem_connect(g_vigem) : VIGEM_ERROR_BUS_NOT_FOUND;
    if (!VIGEM_SUCCESS(error)) {
        std::printf("No se encontro el driver ViGEmBus (error 0x%X). Instala ViGEmBus de Nefarius.\n", error);
        if (g_vigem) vigem_free(g_vigem);
        std::printf("Presiona Enter para salir.");
        std::getchar();
        return 1;
    }

    SDL_SetMainReady();
    SDL_SetHint(SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS, "1");
    SDL_SetHint(SDL_HINT_JOYSTICK_HIDAPI, "1");
    SDL_SetHint(SDL_HINT_JOYSTICK_HIDAPI_PS4, "1");
    SDL_SetHint(SDL_HINT_JOYSTICK_HIDAPI_PS5, "1");
    SDL_SetHint(SDL_HINT_JOYSTICK_HIDAPI_PS4_RUMBLE, "1"); // full reports over Bluetooth: needed for rumble
    SDL_SetHint(SDL_HINT_JOYSTICK_HIDAPI_PS5_RUMBLE, "1");
    SDL_SetHint(SDL_HINT_JOYSTICK_HIDAPI_PS5_PLAYER_LED, "1");
    SDL_SetHint(SDL_HINT_JOYSTICK_HIDAPI_XBOX, "0");       // leave Xbox pads alone
    SDL_SetHint(SDL_HINT_XINPUT_ENABLED, "0");             // never read the virtual pads back
    SDL_SetHint(SDL_HINT_JOYSTICK_RAWINPUT, "0");
    SDL_SetHint(SDL_HINT_JOYSTICK_WGI, "0");

    if (SDL_Init(SDL_INIT_GAMECONTROLLER | SDL_INIT_EVENTS) != 0) {
        std::printf("SDL no inicio: %s\n", SDL_GetError());
        vigem_disconnect(g_vigem);
        vigem_free(g_vigem);
        return 1;
    }

    std::printf("Esperando controles PlayStation (USB o Bluetooth)...\n");

    while (g_running) {
        SDL_Event event;
        while (SDL_PollEvent(&event)) {
            if (event.type == SDL_CONTROLLERDEVICEADDED)
                AddPad(event.cdevice.which);
            else if (event.type == SDL_CONTROLLERDEVICEREMOVED)
                RemovePad(event.cdevice.which);
            else if (event.type == SDL_QUIT)
                g_running = false;
        }
        for (auto& [id, pad] : g_pads)
            UpdatePad(*pad);
        SDL_Delay(1);
    }

    std::printf("\nCerrando: quitando los controles virtuales...\n");
    while (!g_pads.empty())
        RemovePad(g_pads.begin()->first);
    SDL_Quit();
    vigem_disconnect(g_vigem);
    vigem_free(g_vigem);
    g_cleanedUp = true;
    return 0;
}
