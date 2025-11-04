# Contributing
Thanks for helping out! Before you get started, here are a few guidelines to follow:

## Issue Guidelines
### Basic Information
When you file an issue, please provide the following information:
- Operating system
- Rust version (if you built it yourself)
- Hummingbird commit hash
  - You can find this by clicking the "Hummingbird" in the top left on Windows or Linux, or by going to the "Hummingbird" menu in macOS and selecting "About Hummingbird". It's next to the version number.
- Hardware information
  - CPU model
  - RAM size
  - Disk type and size
  - Graphics card model (including integrated)

### Reproducability
When reporting an issue, please provide minimal steps that reproduce the issue. **If reproducing your issue requires distributing copyrighted material, please *DO NOT* attach this material to your Github issue.** Create the issue first, then we can establish further steps.

## PR Guidelines
### Code Style
Try not to introduce any warnings (including clippy lints). Dead code warnings are acceptable, if there is a reasonable possibility the dead code would be required in the near future. Ensure your code has been formatted correctly before submitting - you can do this with `cargo fmt`.

### Documentation
If you introduce a new component that has a lot of moving pieces, or will be a critical part of the application, make sure it's well documented. For example, consider the Device traits (in `src/devices/traits.rs`). Since this code is likely to interact with other components, and the code is likely to be used by other people, it's important to document it well.

Note that not all code in Hummingbird is documented this extensively. If you think your code is self-explanatory (i.e. commenting it would essentially result in simply explaining that the line of code performs the action that it self-evidently peforms anyways), don't over-comment it. Instead, focus on writing clear and concise code that is easy to understand. However, if you do something that might have more opaque reasoning or flow, consider adding a comment to explain it.

### Platform Support
It is expected that new code works on all three major platforms (Linux, macOS, and Windows). However, while you should test on all platforms available to you, I am aware that not everyone has the hardware (or time) to do this.

If you think it's likely that your code will not work on a particular platform, please reach out to me (@143mailliw) when opening your pull request. I can fill in the gaps.
