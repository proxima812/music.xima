import assert from 'node:assert/strict'
import test from 'node:test'

import {
  DECK_COMMIT_FRACTION,
  DECK_EDGE_DRAG_FRACTION,
  DECK_MAX_DRAG_FRACTION,
  clampDeckDrag,
  deckNeighbors,
  shouldCommitDeckSwipe,
} from './deck.ts'

test('deckNeighbors returns null neighbors for a null or invalid index', () => {
  const items = ['first', 'second']

  assert.deepEqual(deckNeighbors(items, null), { previous: null, current: null, next: null })
  assert.deepEqual(deckNeighbors(items, -1), { previous: null, current: null, next: null })
  assert.deepEqual(deckNeighbors(items, 2), { previous: null, current: null, next: null })
  assert.deepEqual(deckNeighbors(items, Number.NaN), { previous: null, current: null, next: null })
})

test('deckNeighbors exposes only the available neighbors without wrapping', () => {
  const items = ['first', 'second', 'third']

  assert.deepEqual(deckNeighbors(items, 0), {
    previous: null,
    current: 'first',
    next: 'second',
  })
  assert.deepEqual(deckNeighbors(items, 2), {
    previous: 'second',
    current: 'third',
    next: null,
  })
})

test('clampDeckDrag applies resistance after the maximum drag fraction', () => {
  const width = 1_000
  const maximumDrag = width * DECK_MAX_DRAG_FRACTION

  assert.equal(clampDeckDrag(maximumDrag, width), maximumDrag)
  assert.equal(clampDeckDrag(maximumDrag + 100, width), maximumDrag + 35)
  assert.equal(clampDeckDrag(-maximumDrag - 100, width), -maximumDrag - 35)
})

test('clampDeckDrag returns neutral drag for invalid dimensions', () => {
  assert.equal(clampDeckDrag(10, 0), 0)
  assert.equal(clampDeckDrag(10, Number.NaN), 0)
  assert.equal(clampDeckDrag(Number.NaN, 100), 0)
})

test('shouldCommitDeckSwipe commits at the width threshold or a fast same-direction flick', () => {
  const width = 1_000
  const threshold = width * DECK_COMMIT_FRACTION

  assert.equal(shouldCommitDeckSwipe(threshold, 0, width), true)
  assert.equal(shouldCommitDeckSwipe(-threshold, 0, width), true)
  assert.equal(shouldCommitDeckSwipe(threshold - 1, 0, width), false)
  assert.equal(shouldCommitDeckSwipe(20, 1, width), true)
  assert.equal(shouldCommitDeckSwipe(-20, -1, width), true)
  assert.equal(shouldCommitDeckSwipe(20, -1, width), false)
})

test('shouldCommitDeckSwipe returns false for invalid dimensions', () => {
  assert.equal(shouldCommitDeckSwipe(300, 1, 0), false)
  assert.equal(shouldCommitDeckSwipe(300, 1, Number.NaN), false)
  assert.equal(shouldCommitDeckSwipe(Number.NaN, 1, 1_000), false)
})

test('deckNeighbors wraps around the ends when the whole queue repeats', () => {
  const items = ['first', 'second', 'third']

  assert.deepEqual(deckNeighbors(items, 0, true), {
    previous: 'third',
    current: 'first',
    next: 'second',
  })
  assert.deepEqual(deckNeighbors(items, 2, true), {
    previous: 'second',
    current: 'third',
    next: 'first',
  })
})

test('deckNeighbors never makes a single track its own neighbor', () => {
  assert.deepEqual(deckNeighbors(['only'], 0, true), {
    previous: null,
    current: 'only',
    next: null,
  })
})

test('clampDeckDrag stops short at the edge of the queue', () => {
  const width = 1_000
  const edgeDrag = width * DECK_EDGE_DRAG_FRACTION

  assert.equal(clampDeckDrag(edgeDrag, width, false), edgeDrag)
  // Дальше края палец тянет почти впустую: 100 px превращаются в 12.
  assert.equal(clampDeckDrag(edgeDrag + 100, width, false), edgeDrag + 12)
  assert.equal(clampDeckDrag(-edgeDrag - 100, width, false), -edgeDrag - 12)
  // Тот же жест при наличии соседа уезжает во много раз дальше.
  assert.ok(Math.abs(clampDeckDrag(400, width, true)) > Math.abs(clampDeckDrag(400, width, false)))
})

test('shouldCommitDeckSwipe never commits without an adjacent track', () => {
  const width = 1_000

  assert.equal(shouldCommitDeckSwipe(width, 0, width, false), false)
  assert.equal(shouldCommitDeckSwipe(20, 1, width, false), false)
})
