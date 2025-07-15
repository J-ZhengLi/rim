# Setup

Upon launching the program for the first time, it will enter setup mode, which will guide you through installing Rust on your environment.

## 1. Start Installation

Once you are in the setup mode, click <button class="primary-button">Install</button> on the home page.

Alternatively, if you have a toolkit manifest, you may click on <u>Install Using Toolkit Manifest</u> label located at the bottom of the home page. Doing so will overrides the built-in default toolkit manifest.

## 2. Configuration

In this step, you may configure your Rust installation, starting by setting an installation path, this is where RIM will store most of the files, including the binary of itself.

You'll see a input bar with a <button class="primary-button">Select folder</button> button at the right, this is where you can input the installation path or select one using file-picker:

<img src="/help/en-US/images/file-picker.png" width="100%"/>

You may also check advanced options by clicking on the `Advanced Options ▼` label, which will expands a whole section containing advanced config options.
You can hover your mouse on each option to see the hint, these options are not recommended to tinker with if you don't know what they does. 

> Some options might be disabled under `Source Configuration`, this means that they are enforced by the toolkit manifest, and it's completely normal.

## 3. Customization

### 3.1 Profile Selection

Components are grouped into two profiles: `Minimal`, `Standard`.
Click to choose what suits you the most, or click `Customize` for advanced component selection.

### 3.2 Component Selection

If you choose `Customize` on profile selection page, you'll be navigated to the component selection page, which looks like this:

<img src="/help/en-US/images/component-select.png" width="100%"/>

Click on the checkbox on the left to select component to install, and click on the name of component to view its detailed description on the right side panel.

### 3.3 Package Source Override

Some components allow you to override it's package source, such as `VS Build Tools`.

In this case, you can either:

1. Continue with the default (official) source.
2. Provided your own package, which can be an local file path or web URL.

<img src="/help/en-US/images/vsbt-src-ovr.png" width="100%"/>

## 4. Confirmation

Before starting installation, RIM will ask you to review your install configuration one last time.

Carefully review the installation path, and the components you are about to install, then press <button class="primary-button">Install</button> to start installation.

## 5. Installation Progress

During installation, a progress bar will shown the overall installation progress, you may also click on `Show Details ▼` to see detailed output.

<img src="/help/en-US/images/start-install.png" width="100%"/>

## 6. Finish

This is the final step of setup process, before the app closes itself, it will ask you whether you like to:

1. Open after finish
    
    Re-open in manager mode after exiting setup.
2. Create desktop shortcut

    Create a desktop shortcut for RIM.
    > On Linux platform, an application shortcut will also be created.

You may check any of the options before pressing <button class="primary-button">Finish</button> to exit the program.

<img src="/help/en-US/images/install-finish.png" width="100%"/>

<style>
.primary-button {
    color: white;
    background-color: #5b98d8;
    margin-inline: 5px;
    border: none;
    border-radius: 20px;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, .4);
    font-weight: bold;
    white-space: nowrap;
    overflow: hidden;
    padding: 3px 10px;
}
</style>
