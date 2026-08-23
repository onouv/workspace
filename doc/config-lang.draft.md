window
  name: root
  path: .
  command: ls
window
  name: mvp
  path: ./mvp
  command: ls
  pane
    pos: left
    id: git
    command: git status
  pane
    pos: right
    id: NATS
    pane
      pos: top
      id: receive
      command: ~/.bin/nats_observe
      env: NATS_SERVER=localhost::4532
    pane
      pos: bottom
      id: send
      command: ~/.bin/nats_send
      env: NATS_SERVER=localhost::4532
