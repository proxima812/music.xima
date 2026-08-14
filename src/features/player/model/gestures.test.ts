import assert from 'node:assert/strict'
import test from 'node:test'

import { shouldCommitDeckSwipe } from './deck.ts'
import { createSwipe, type SwipeDelta, type SwipeDirection, type SwipeEnd } from './gestures.ts'

class TestElement extends EventTarget {
  closest(selector: string): TestElement | null {
    return selector === '[data-no-swipe]' ? this : null
  }
}

Object.defineProperty(globalThis, 'Element', { value: TestElement, configurable: true })

type Recorder = {
  ends: number
  moves: SwipeDelta[]
  swipes: SwipeDirection[]
}

function createRecorder(): Recorder {
  return { ends: 0, moves: [], swipes: [] }
}

function touchEvent(
  x: number,
  y: number,
  timeStamp: number,
  target: EventTarget | null = null,
): TouchEvent {
  return {
    changedTouches: [{ clientX: x, clientY: y }],
    target,
    timeStamp,
    touches: [{ clientX: x, clientY: y }],
  } as unknown as TouchEvent
}

function createHandlers(recorder: Recorder, directions: readonly SwipeDirection[]) {
  return createSwipe({
    directions,
    onEnd: () => {
      recorder.ends += 1
    },
    onMove: (delta) => {
      recorder.moves.push(delta)
    },
    onSwipe: (direction) => {
      recorder.swipes.push(direction)
    },
  })
}

test('movement below the axis-lock slop remains pending', () => {
  const recorder = createRecorder()
  const swipe = createHandlers(recorder, ['left', 'down'])

  swipe.onTouchStart(touchEvent(100, 100, 0))
  swipe.onTouchMove(touchEvent(105, 104, 10))
  swipe.onTouchEnd(touchEvent(105, 104, 20))

  assert.deepEqual(recorder.moves, [])
  assert.deepEqual(recorder.swipes, [])
  assert.equal(recorder.ends, 1)
})

test('axis dominance locks horizontal and vertical gestures to one action', () => {
  const horizontal = createRecorder()
  const horizontalSwipe = createHandlers(horizontal, ['left', 'down'])
  horizontalSwipe.onTouchStart(touchEvent(100, 100, 0))
  horizontalSwipe.onTouchMove(touchEvent(25, 50, 10))
  horizontalSwipe.onTouchEnd(touchEvent(25, 50, 20))

  const vertical = createRecorder()
  const verticalSwipe = createHandlers(vertical, ['left', 'down'])
  verticalSwipe.onTouchStart(touchEvent(100, 100, 0))
  verticalSwipe.onTouchMove(touchEvent(50, 175, 10))
  verticalSwipe.onTouchEnd(touchEvent(50, 175, 20))

  assert.deepEqual(horizontal.moves, [{ dx: -75, dy: 0 }])
  assert.deepEqual(horizontal.swipes, ['left'])
  assert.deepEqual(vertical.moves, [{ dx: 0, dy: 75 }])
  assert.deepEqual(vertical.swipes, ['down'])
})

test('left, right, and down each resolve at the configured threshold', () => {
  const left = createRecorder()
  const leftSwipe = createHandlers(left, ['left'])
  leftSwipe.onTouchStart(touchEvent(100, 100, 0))
  leftSwipe.onTouchEnd(touchEvent(40, 100, 10))

  const right = createRecorder()
  const rightSwipe = createHandlers(right, ['right'])
  rightSwipe.onTouchStart(touchEvent(100, 100, 0))
  rightSwipe.onTouchEnd(touchEvent(160, 100, 10))

  const down = createRecorder()
  const downSwipe = createHandlers(down, ['down'])
  downSwipe.onTouchStart(touchEvent(100, 100, 0))
  downSwipe.onTouchEnd(touchEvent(100, 160, 10))

  assert.deepEqual(left.swipes, ['left'])
  assert.deepEqual(right.swipes, ['right'])
  assert.deepEqual(down.swipes, ['down'])
})

test('a horizontal gesture can commit with deck width and velocity', () => {
  const recorder = createRecorder()
  const swipe = createSwipe({
    directions: ['left', 'right'],
    onSwipe: (direction) => {
      recorder.swipes.push(direction)
    },
    shouldCommit: (_direction, end) => shouldCommitDeckSwipe(end.dx, end.velocityX, 1_000),
  })

  swipe.onTouchStart(touchEvent(100, 100, 0))
  swipe.onTouchMove(touchEvent(85, 100, 10))
  swipe.onTouchEnd(touchEvent(75, 100, 20))

  assert.deepEqual(recorder.swipes, ['left'])
})

test('a data-no-swipe gesture start is ignored', () => {
  const recorder = createRecorder()
  const swipe = createHandlers(recorder, ['left'])

  swipe.onTouchStart(touchEvent(100, 100, 0, new TestElement()))
  swipe.onTouchMove(touchEvent(0, 100, 10))
  swipe.onTouchEnd(touchEvent(0, 100, 20))

  assert.deepEqual(recorder.moves, [])
  assert.deepEqual(recorder.swipes, [])
  assert.equal(recorder.ends, 0)
})

test('cancel ends an active gesture once without swiping', () => {
  const recorder = createRecorder()
  const swipe = createHandlers(recorder, ['left'])

  swipe.onTouchStart(touchEvent(100, 100, 0))
  swipe.onTouchMove(touchEvent(0, 100, 10))
  swipe.onTouchCancel(touchEvent(0, 100, 20))
  swipe.onTouchCancel(touchEvent(0, 100, 30))

  assert.equal(recorder.ends, 1)
  assert.deepEqual(recorder.swipes, [])
})

test('an invalid velocity sample is reported as zero', () => {
  let receivedEnd: SwipeEnd | null = null
  const swipe = createSwipe({
    directions: ['left'],
    onSwipe: () => {},
    shouldCommit: (_direction, end) => {
      receivedEnd = end
      return false
    },
  })

  swipe.onTouchStart(touchEvent(100, 100, 100))
  swipe.onTouchMove(touchEvent(85, 100, 110))
  swipe.onTouchEnd(touchEvent(75, 100, 110))

  assert.deepEqual(receivedEnd, { dx: -25, dy: 0, velocityX: 0, velocityY: 0 })
})
