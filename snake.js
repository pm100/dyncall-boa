// Snake Slideshow — SDL2 controlled entirely from JavaScript via dyncall.
//
// Run from the repo root with:
//   cargo run --bin demo -- snake.js

// ── SDL / image / sound function bindings ─────────────────────────────────────
const SDL_Init      = exfun("C:\\tools\\SDL2.dll|SDL_Init|u32|i32|");
const SDL_Quit      = exfun("C:\\tools\\SDL2.dll|SDL_Quit||void|");
const SDL_GetError  = exfun("C:\\tools\\SDL2.dll|SDL_GetError||cstr|");

const SDL_CreateWindow   = exfun("C:\\tools\\SDL2.dll|SDL_CreateWindow|cstr,i32,i32,i32,i32,u32|ptr|");
const SDL_CreateRenderer = exfun("C:\\tools\\SDL2.dll|SDL_CreateRenderer|ptr,i32,u32|ptr|");
const SDL_RenderClear    = exfun("C:\\tools\\SDL2.dll|SDL_RenderClear|ptr|i32|");
const SDL_RenderCopy     = exfun("C:\\tools\\SDL2.dll|SDL_RenderCopy|ptr,ptr,ptr,ptr|i32|");
const SDL_RenderPresent  = exfun("C:\\tools\\SDL2.dll|SDL_RenderPresent|ptr|void|");
const SDL_Delay          = exfun("C:\\tools\\SDL2.dll|SDL_Delay|u32|void|");

const SDL_CreateTextureFromSurface = exfun("C:\\tools\\SDL2.dll|SDL_CreateTextureFromSurface|ptr,ptr|ptr|");
const SDL_DestroyTexture           = exfun("C:\\tools\\SDL2.dll|SDL_DestroyTexture|ptr|void|");
const SDL_FreeSurface              = exfun("C:\\tools\\SDL2.dll|SDL_FreeSurface|ptr|void|");

const IMG_Init = exfun("C:\\tools\\SDL2_image.dll|IMG_Init|i32|i32|");
const IMG_Load = exfun("C:\\tools\\SDL2_image.dll|IMG_Load|cstr|ptr|");

const PlaySound = exfun("winmm.dll|PlaySoundA|cstr,ptr,u32|i32|");

// ── SDL constants ─────────────────────────────────────────────────────────────
const SDL_INIT_VIDEO            = 0x00000020;
const SDL_WINDOWPOS_CENTERED    = 0x2FFF0000;
const SDL_WINDOW_SHOWN          = 0x00000004;
const SDL_RENDERER_ACCELERATED  = 0x00000002;
const IMG_INIT_JPG              = 0x00000001;

// SND_ALIAS (0x00010000) | SND_ASYNC (0x0001) | SND_NODEFAULT (0x0002)
const SND_FLAGS = 0x00010003;

const SLIDE_MS = 3000;  // milliseconds per slide
const LOOPS    = 2;     // how many times to cycle through all images

// ── Image paths (relative to working directory) ───────────────────────────────
const images = [
    "assets\\snake_0.jpg",
    "assets\\snake_1.jpg",
    "assets\\snake_2.jpg",
    "assets\\snake_3.jpg",
    "assets\\snake_4.jpg",
    "assets\\snake_5.jpg",
];

// ── Init ──────────────────────────────────────────────────────────────────────
const initResult = SDL_Init(SDL_INIT_VIDEO);
if (initResult !== 0) {
    console.log("SDL_Init failed:", SDL_GetError());
    throw new Error("SDL_Init failed");
}
IMG_Init(IMG_INIT_JPG);

const win = SDL_CreateWindow(
    "Snake Slideshow  [dyncall + Boa]",
    SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
    800, 600,
    SDL_WINDOW_SHOWN
);
if (win === 0) {
    console.log("SDL_CreateWindow failed:", SDL_GetError());
    throw new Error("SDL_CreateWindow failed");
}

const ren = SDL_CreateRenderer(win, -1, SDL_RENDERER_ACCELERATED);
if (ren === 0) {
    console.log("SDL_CreateRenderer failed:", SDL_GetError());
    throw new Error("SDL_CreateRenderer failed");
}

console.log("Window and renderer ready. Starting slideshow...");

// ── Slide loop ────────────────────────────────────────────────────────────────
outer: for (let loop = 0; loop < LOOPS; loop++) {
    for (let i = 0; i < images.length; i++) {
        const path = images[i];
        console.log(`  Slide ${loop * images.length + i + 1}: ${path}`);

        const surface = IMG_Load(path);
        if (surface === 0) {
            console.log("  IMG_Load failed:", SDL_GetError(), "- skipping");
            continue;
        }

        const texture = SDL_CreateTextureFromSurface(ren, surface);
        SDL_FreeSurface(surface);

        if (texture === 0) {
            console.log("  SDL_CreateTextureFromSurface failed:", SDL_GetError(), "- skipping");
            continue;
        }

        SDL_RenderClear(ren);
        SDL_RenderCopy(ren, texture, 0, 0);  // 0 = NULL ptr → full window rect
        SDL_RenderPresent(ren);

        // Play a Windows system sound (non-blocking)
        PlaySound("SystemAsterisk", 0, SND_FLAGS);

        // Delay in 100ms chunks; poll for SDL_QUIT each tick so the
        // window's close button works immediately.
        const ticks = Math.floor(SLIDE_MS / 100);
        for (let t = 0; t < ticks; t++) {
            if (checkQuit()) {
                SDL_DestroyTexture(texture);
                break outer;
            }
            SDL_Delay(100);
        }

        SDL_DestroyTexture(texture);
    }
}

// ── Cleanup ───────────────────────────────────────────────────────────────────
console.log("Slideshow complete.");
SDL_Quit();
