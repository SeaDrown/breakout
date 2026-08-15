# Breakout* with Bevy

Bevy is an open-source, code-first Rust based game engine, and I have spent the last 2 days working on this project with Bevy. I really enjoy using Bevy so far, in particular I am having a great time with its brilliant Entity Component System.

This is a project that I paraphrased from Bevy's example of an implementation of **Atari Breakout**. I effectively copied the logic (barring the destructible blocks being spawned), but tidied it up by using my own intuition in parts of the code where I believe the bevy implementation was unclear/unintuitive, and added many (many) comments to help people understand what is happening. I will be working on refactoring this entirely, adding mechanics to change the paddle size, spawn multiple balls, and perhaps some sort of game menu, so I am excited to continue to use Bevy in the future.

Since this is the project source, and **not** an executable, you'll have to compile this using Cargo after downloading. Cargo.toml has some optimisations that need to be cleared to ensure that the executable generated, like dynamic linking needs to be removed.
