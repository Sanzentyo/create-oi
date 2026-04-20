/**
 * Compatibility shim for Boost 1.85+ where io_service and deadline_timer
 * have been removed.
 */

#ifndef BOOST_COMPAT_H
#define BOOST_COMPAT_H

#include <boost/version.hpp>

#if BOOST_VERSION >= 108500

#include <boost/asio/io_context.hpp>
#include <boost/asio/steady_timer.hpp>
#include <chrono>

// io_service was removed; provide a subclass of io_context that re-adds
// the removed `reset()` method and preserves member-function-pointer casts
// like `&boost::asio::io_service::run`.
namespace boost { namespace asio {
  class io_service : public io_context {
  public:
    using io_context::io_context;
    void reset() { restart(); }
  };
}}

// deadline_timer was removed; provide a subclass of steady_timer that
// accepts `expires_from_now()` calls with chrono durations.
namespace boost { namespace asio {
  class deadline_timer : public steady_timer {
  public:
    using steady_timer::steady_timer;
    template<typename Duration>
    void expires_from_now(const Duration& d) {
      expires_after(d);
    }
  };
}}

// posix_time::milliseconds → std::chrono::milliseconds
namespace boost { namespace posix_time {
  inline std::chrono::milliseconds milliseconds(long ms) {
    return std::chrono::milliseconds(ms);
  }
}}

#endif // BOOST_VERSION >= 108500

#endif // BOOST_COMPAT_H
