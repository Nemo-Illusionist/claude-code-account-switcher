Output "demo.gif"  # rendered with: vhs assets/demo.tape
Set Shell zsh
Set FontSize 20
Set Width 1160
Set Height 560
Set Padding 24
Set Theme "Catppuccin Mocha"
Set TypingSpeed 45ms

Hide
Type "exec env HOME='$DEMO_HOME' ZDOTDIR='$DEMO_HOME' zsh -i" Enter
Sleep 2s
Type "cd ~ && clear" Enter
Sleep 1s
Show
Sleep 1s

Type "# accounts are bound to directories — no global switch" Enter
Sleep 1200ms
Type "claude-acc list" Enter
Sleep 2500ms

Type "cd work/api-gateway" Enter
Sleep 800ms
Type "claude-acc status" Enter
Sleep 2200ms
Type 'echo ${CLAUDE_CONFIG_DIR#$HOME/}' Enter
Sleep 2200ms

Type "cd ~/personal/side-project" Enter
Sleep 800ms
Type "claude-acc status" Enter
Sleep 2200ms
Type 'echo ${CLAUDE_CONFIG_DIR#$HOME/}' Enter
Sleep 3000ms
