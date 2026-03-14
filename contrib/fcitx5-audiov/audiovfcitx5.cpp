#include <string>
#include <memory>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/instance.h>
#include <fcitx-utils/event.h>
#include <fcitx-utils/dbus/objectvtable.h>
#include <fcitx-utils/dbus/bus.h>
#include <fcitx-module/dbus/dbus_public.h>

namespace {

constexpr const char *kObjectPath = "/org/freedesktop/Fcitx5/Audiov";
constexpr const char *kInterface = "org.fcitx.Fcitx5.Audiov1";

class AudiovBridge final : public fcitx::AddonInstance,
                           public fcitx::dbus::ObjectVTable<AudiovBridge> {
public:
    explicit AudiovBridge(fcitx::Instance *instance) : instance_(instance) {
        deferred_registration_ = instance_->eventLoop().addDeferEvent(
            [this](fcitx::EventSource *) {
                tryRegister();
                return false;
            });
    }

    bool CommitText(const std::string &text) {
        if (!registered_ || text.empty()) {
            return false;
        }

        auto *inputContext = instance_->inputContextManager().lastFocusedInputContext();
        if (!inputContext) {
            inputContext = instance_->inputContextManager().mostRecentInputContext();
        }
        if (!inputContext) {
            return false;
        }

        inputContext->commitString(text);
        return true;
    }

private:
    void tryRegister() {
        if (registered_) {
            return;
        }

        auto *dbusAddon = instance_->addonManager().addon("dbus", true);
        if (!dbusAddon) {
            return;
        }

        auto *bus = dbusAddon->call<fcitx::IDBusModule::bus>();
        if (!bus) {
            return;
        }

        registered_ = bus->addObjectVTable(kObjectPath, kInterface, *this);
    }

    fcitx::Instance *instance_;
    bool registered_ = false;
    std::unique_ptr<fcitx::EventSource> deferred_registration_;

    FCITX_OBJECT_VTABLE_METHOD(CommitText, "CommitText", "s", "b");
};

class AudiovBridgeFactory final : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
        return new AudiovBridge(manager->instance());
    }
};

} // namespace

FCITX_ADDON_FACTORY(AudiovBridgeFactory)
