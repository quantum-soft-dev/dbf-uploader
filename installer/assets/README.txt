Data Exporter Installer Assets
===============================

This directory contains assets for the MSI installer.

Files:
------
- license.rtf     - License agreement displayed during installation
- app.ico         - Application icon (optional, add your own)

Adding an Application Icon:
---------------------------
1. Create or obtain a Windows .ico file (256x256 recommended)
2. Save it as "app.ico" in this directory
3. Uncomment the Icon lines in installer/wix/main.wxs:
   - <Icon Id="AppIcon.ico" ...>
   - <Property Id="ARPPRODUCTICON" ...>
4. Rebuild the installer

The icon will appear in:
- Add/Remove Programs list
- Start Menu shortcuts
