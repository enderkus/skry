# Records docs/demo.gif with https://github.com/charmbracelet/vhs.
# `skry demo` renders a synthetic fleet, so no servers are needed:
#   vhs docs/demo.tape
Output docs/demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 720
Set Padding 12
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Hide
Type "export PATH=$PWD/target/release:$PATH && clear"
Enter
Show

Type "skry demo"
Sleep 400ms
Enter
Sleep 3s
Right
Sleep 600ms
Right
Sleep 1s
Enter
Sleep 3s
Tab
Sleep 2s
Tab
Sleep 1.5s
Tab
Sleep 2s
Tab
Sleep 2s
Escape
Sleep 1s
Type "S"
Sleep 3s
Type "S"
Sleep 800ms
Type "/web"
Sleep 600ms
Enter
Sleep 2s
Escape
Sleep 800ms
Type "?"
Sleep 2.5s
Type "q"
Sleep 500ms
Type "q"
Sleep 300ms
