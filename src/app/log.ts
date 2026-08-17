//! Logging.

import pino from "pino";
import pretty from "pino-pretty";

/**
 * Formatted lines on a terminal, JSON everywhere else.
 *
 * Redirecting or piping standard error yields machine-readable records, which
 * is what anything consuming the log wants. `NO_COLOR` is honoured.
 */
const stream = () =>
    process.stderr.isTTY
        ? pretty({
              destination: 2,
              colorize: !process.env.NO_COLOR,
              ignore: "pid,hostname",
              translateTime: "SYS:HH:MM:ss",
          })
        : pino.destination(2);

/** Global logger, writing to standard error. */
export const log = pino({ level: "info" }, stream());

/** Raises the log level by the given number of steps, or lowers it if negative. */
export function init(vlevel: number) {
    const level = pino.levels.values.info! - vlevel * 10;
    log.level = pino.levels.labels[level] ?? (vlevel > 0 ? "trace" : "silent");
}
